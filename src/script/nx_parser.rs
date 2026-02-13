//! # Nx Monorepo Parser
//!
//! This module parses Nx workspace configurations and extracts available project targets.
//!
//! ## Overview
//!
//! This parser invokes the Nx CLI to discover projects and their targets. It supports
//! both locally installed (`npx nx`) and globally installed (`nx`) binaries.
//!
//! ## Key Types
//!
//! - [`NxTarget`] - Represents an Nx project target with display metadata for the TUI
//! - [`is_nx_available`] - Checks if the `nx` CLI is available (local or global)
//! - [`list_targets`] - Main function to list all project targets from an Nx workspace
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! npx nx show projects
//! npx nx show project <project-name> --json
//! ```
//!
//! To discover projects and their targets (build, test, lint, serve, etc.).
//!
//! ## Execution
//!
//! Targets are executed using the `project:target` format:
//! ```bash
//! npx nx run <project>:<target>
//! ```
//!
//! ## Availability Caching
//!
//! The Nx binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::script::discovery::format_display_name;

/// Cache for nx availability check (checked once per process)
static NX_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Cache for whether npx nx (local) is preferred over global nx
static NX_USE_NPX: OnceLock<bool> = OnceLock::new();

/// Nx target item for TUI display (mirrors other script types)
#[derive(Debug, Clone)]
pub struct NxTarget {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
    pub project: String,
    pub target: String,
}

/// Check if `nx` is available (either via `npx` or globally).
///
/// Checks for local `npx nx` first, then falls back to global `nx`.
pub fn is_nx_available() -> bool {
    *NX_AVAILABLE.get_or_init(|| {
        // Check for local nx via npx first
        // stdin must be null to prevent npx from prompting to install nx
        let npx_available = Command::new("npx")
            .args(["nx", "--version"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if npx_available {
            return true;
        }

        // Fall back to global nx
        Command::new("nx")
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Determine the Nx command to use (npx nx or nx).
///
/// Returns `("npx", vec!["nx"])` for local or `("nx", vec![])` for global.
/// Caches the result to avoid repeated subprocess spawns.
pub fn nx_command() -> (&'static str, Vec<&'static str>) {
    let use_npx = *NX_USE_NPX.get_or_init(|| {
        Command::new("npx")
            .args(["nx", "--version"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });

    if use_npx {
        ("npx", vec!["nx"])
    } else {
        ("nx", vec![])
    }
}

/// Parse `nx show project <name> --json` output to extract targets for a single project.
///
/// The JSON output contains a `targets` object where each key is a target name
/// and the value contains target configuration.
///
/// Each project gets its own category formatted as `nx:<workspace>/<project>`
/// so projects appear as separate groups in the TUI.
fn parse_project_targets(
    output: &str,
    project: &str,
    workspace_name: &str,
) -> Result<Vec<NxTarget>> {
    let project_info: Value =
        serde_json::from_str(output).context("Failed to parse nx project JSON")?;

    let mut targets = Vec::new();

    let targets_obj = match project_info["targets"].as_object() {
        Some(t) => t,
        None => return Ok(targets),
    };

    // Each project gets its own category so targets are grouped per-project
    let project_category = format!("nx:{}:{}", workspace_name, project);

    for (target_name, _target_config) in targets_obj {
        let qualified_name = format!("{}:{}", project, target_name);
        let display_name = format_display_name(target_name);
        let description = format!("nx run {}:{}", project, target_name);

        targets.push(NxTarget {
            name: qualified_name,
            display_name,
            category: project_category.clone(),
            description,
            emoji: Some("\u{1f537}".to_string()), // 🔷
            ignored: false,
            project: project.to_string(),
            target: target_name.to_string(),
        });
    }

    targets.sort_by(|a, b| a.target.cmp(&b.target));
    Ok(targets)
}

/// List all projects in an Nx workspace.
///
/// Runs `nx show projects` in the workspace directory and returns project names.
fn list_projects(workspace_dir: &Path) -> Result<Vec<String>> {
    let (cmd, mut base_args) = nx_command();
    base_args.extend(["show", "projects"]);

    let output = Command::new(cmd)
        .args(&base_args)
        .current_dir(workspace_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Failed to run nx show projects in: {}",
                workspace_dir.display()
            )
        })?;

    if !output.status.success() {
        anyhow::bail!("nx show projects failed in {}", workspace_dir.display(),);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let projects: Vec<String> = output_str
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(projects)
}

/// Get targets for a specific project.
///
/// Runs `nx show project <name> --json` in the workspace directory.
fn get_project_targets(
    workspace_dir: &Path,
    project: &str,
    workspace_name: &str,
) -> Result<Vec<NxTarget>> {
    let (cmd, mut base_args) = nx_command();
    base_args.extend(["show", "project", project, "--json"]);

    let output = Command::new(cmd)
        .args(&base_args)
        .current_dir(workspace_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Failed to run nx show project {} in: {}",
                project,
                workspace_dir.display()
            )
        })?;

    if !output.status.success() {
        // Skip projects that fail (e.g., misconfigured)
        return Ok(Vec::new());
    }

    let output_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).to_string(),
    };

    parse_project_targets(&output_str, project, workspace_name)
}

/// List all targets from all projects in an Nx workspace.
///
/// Discovers projects via `nx show projects`, then queries each project
/// for its targets via `nx show project <name> --json` in parallel.
///
/// Each project gets its own category so the TUI groups targets per-project
/// instead of showing a flat list of duplicated target names.
pub fn list_targets(nx_json_path: &Path, category: &str) -> Result<Vec<NxTarget>> {
    let workspace_dir = nx_json_path.parent().with_context(|| {
        format!(
            "Failed to get parent directory of: {}",
            nx_json_path.display()
        )
    })?;

    let projects = list_projects(workspace_dir)?;

    // Query all projects in parallel using threads to avoid the N+1 latency problem.
    // Each `nx show project <name> --json` call spawns a subprocess, so parallelism
    // gives a significant speedup for large monorepos (e.g. 18 microservices).
    let workspace_dir_owned = workspace_dir.to_path_buf();
    let category_owned = category.to_string();

    let handles: Vec<_> = projects
        .into_iter()
        .map(|project| {
            let ws_dir = workspace_dir_owned.clone();
            let ws_name = category_owned.clone();
            thread::spawn(move || get_project_targets(&ws_dir, &project, &ws_name))
        })
        .collect();

    let mut all_targets = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(Ok(targets)) => all_targets.extend(targets),
            Ok(Err(_)) => {
                // Skip projects that fail to parse
                continue;
            }
            Err(_) => {
                // Skip projects whose thread panicked
                continue;
            }
        }
    }

    // Sort by project name, then by target name
    all_targets.sort_by(|a, b| {
        a.project
            .cmp(&b.project)
            .then_with(|| a.target.cmp(&b.target))
    });
    Ok(all_targets)
}

/// Collect per-project category display names from a list of Nx targets.
///
/// Returns a map from category key (e.g. `"nx:monopoly:service.auth"`)
/// to display name (e.g. `"🔷 Service Auth"`).
pub fn collect_category_display_names(
    targets: &[NxTarget],
) -> std::collections::HashMap<String, String> {
    let mut names = std::collections::HashMap::new();
    for target in targets {
        names
            .entry(target.category.clone())
            .or_insert_with(|| format!("🔷 {}", format_display_name(&target.project)));
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project_json() -> String {
        r#"{
            "name": "my-app",
            "root": "apps/my-app",
            "sourceRoot": "apps/my-app/src",
            "targets": {
                "build": {
                    "executor": "@nx/webpack:webpack",
                    "options": {
                        "outputPath": "dist/apps/my-app"
                    }
                },
                "serve": {
                    "executor": "@nx/webpack:dev-server",
                    "options": {
                        "buildTarget": "my-app:build"
                    }
                },
                "test": {
                    "executor": "@nx/jest:jest",
                    "options": {
                        "jestConfig": "apps/my-app/jest.config.ts"
                    }
                },
                "lint": {
                    "executor": "@nx/eslint:lint"
                }
            }
        }"#
        .to_string()
    }

    #[test]
    fn test_parse_project_targets_extracts_all_targets() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        assert_eq!(targets.len(), 4);

        let names: Vec<&str> = targets.iter().map(|t| t.target.as_str()).collect();
        assert!(names.contains(&"build"));
        assert!(names.contains(&"serve"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"lint"));
    }

    #[test]
    fn test_parse_project_targets_qualified_names() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        let build = targets.iter().find(|t| t.target == "build").unwrap();
        assert_eq!(build.name, "my-app:build");

        let serve = targets.iter().find(|t| t.target == "serve").unwrap();
        assert_eq!(serve.name, "my-app:serve");
    }

    #[test]
    fn test_parse_project_targets_descriptions() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        let build = targets.iter().find(|t| t.target == "build").unwrap();
        assert_eq!(build.description, "nx run my-app:build");

        let test = targets.iter().find(|t| t.target == "test").unwrap();
        assert_eq!(test.description, "nx run my-app:test");
    }

    #[test]
    fn test_parse_project_targets_display_names() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        let build = targets.iter().find(|t| t.target == "build").unwrap();
        assert_eq!(build.display_name, "Build");

        let serve = targets.iter().find(|t| t.target == "serve").unwrap();
        assert_eq!(serve.display_name, "Serve");
    }

    #[test]
    fn test_parse_project_targets_category() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "my-workspace").unwrap();

        for target in &targets {
            assert_eq!(target.category, "nx:my-workspace:my-app");
        }
    }

    #[test]
    fn test_parse_project_targets_emoji() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        for target in &targets {
            assert_eq!(target.emoji, Some("\u{1f537}".to_string())); // 🔷
        }
    }

    #[test]
    fn test_parse_project_targets_project_field() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        for target in &targets {
            assert_eq!(target.project, "my-app");
        }
    }

    #[test]
    fn test_parse_project_targets_sort_order() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        // Targets should be sorted alphabetically by target name
        let target_names: Vec<&str> = targets.iter().map(|t| t.target.as_str()).collect();
        let mut sorted = target_names.clone();
        sorted.sort();
        assert_eq!(target_names, sorted);
    }

    #[test]
    fn test_parse_project_targets_empty_targets() {
        let json = r#"{"name": "empty-project", "targets": {}}"#;
        let targets = parse_project_targets(json, "empty-project", "myworkspace").unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_project_targets_no_targets_key() {
        let json = r#"{"name": "no-targets-project", "root": "apps/no-targets"}"#;
        let targets = parse_project_targets(json, "no-targets", "myworkspace").unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_project_targets_invalid_json() {
        let result = parse_project_targets("not json", "bad", "myworkspace");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_project_targets_complex_target_names() {
        let json = r#"{
            "name": "my-lib",
            "targets": {
                "build": {},
                "build-storybook": {},
                "component-test": {},
                "e2e": {}
            }
        }"#;
        let targets = parse_project_targets(json, "my-lib", "myworkspace").unwrap();

        assert_eq!(targets.len(), 4);

        let build_storybook = targets
            .iter()
            .find(|t| t.target == "build-storybook")
            .unwrap();
        assert_eq!(build_storybook.display_name, "Build Storybook");
        assert_eq!(build_storybook.name, "my-lib:build-storybook");

        let component_test = targets
            .iter()
            .find(|t| t.target == "component-test")
            .unwrap();
        assert_eq!(component_test.display_name, "Component Test");
    }

    #[test]
    fn test_parse_project_targets_multiple_projects() {
        let json_app = r#"{"name": "app", "targets": {"build": {}, "serve": {}}}"#;
        let json_lib = r#"{"name": "lib", "targets": {"build": {}, "test": {}}}"#;

        let app_targets = parse_project_targets(json_app, "app", "ws").unwrap();
        let lib_targets = parse_project_targets(json_lib, "lib", "ws").unwrap();

        assert_eq!(app_targets.len(), 2);
        assert_eq!(lib_targets.len(), 2);

        // Verify distinct project fields
        for t in &app_targets {
            assert_eq!(t.project, "app");
            assert_eq!(t.category, "nx:ws:app");
        }
        for t in &lib_targets {
            assert_eq!(t.project, "lib");
            assert_eq!(t.category, "nx:ws:lib");
        }
    }

    #[test]
    fn test_parse_project_targets_per_project_categories() {
        let json_app = r#"{"name": "app", "targets": {"build": {}, "test": {}, "lint": {}}}"#;
        let json_lib = r#"{"name": "lib", "targets": {"build": {}, "test": {}, "lint": {}}}"#;

        let app_targets = parse_project_targets(json_app, "app", "monopoly").unwrap();
        let lib_targets = parse_project_targets(json_lib, "lib", "monopoly").unwrap();

        // Even though both have the same target names, they should have different categories
        assert_ne!(app_targets[0].category, lib_targets[0].category);
        assert_eq!(app_targets[0].category, "nx:monopoly:app");
        assert_eq!(lib_targets[0].category, "nx:monopoly:lib");
    }

    #[test]
    fn test_nx_target_not_ignored() {
        let json = sample_project_json();
        let targets = parse_project_targets(&json, "my-app", "myworkspace").unwrap();

        for target in &targets {
            assert!(!target.ignored);
        }
    }

    #[test]
    fn test_collect_category_display_names() {
        let json_app = r#"{"name": "app", "targets": {"build": {}}}"#;
        let json_lib = r#"{"name": "lib", "targets": {"test": {}}}"#;

        let mut all_targets = parse_project_targets(json_app, "service.auth", "monopoly").unwrap();
        all_targets.extend(parse_project_targets(json_lib, "service.api", "monopoly").unwrap());

        let names = collect_category_display_names(&all_targets);
        assert_eq!(
            names.get("nx:monopoly:service.auth"),
            Some(&"🔷 Service Auth".to_string())
        );
        assert_eq!(
            names.get("nx:monopoly:service.api"),
            Some(&"🔷 Service Api".to_string())
        );
    }
}
