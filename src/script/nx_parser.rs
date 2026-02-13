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
//! The parser runs a single command to fetch the entire project graph:
//! ```bash
//! npx nx graph --file=stdout
//! ```
//!
//! This returns all projects and their targets in one call, avoiding the N+1
//! subprocess problem that occurs when querying each project individually.
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

/// Fetch the full project graph from an Nx workspace in a single CLI call.
///
/// Runs `nx graph --file=stdout` which returns all projects with their targets
/// in one JSON payload. This avoids the N+1 subprocess problem where each project
/// would otherwise require a separate `nx show project <name> --json` call.
fn fetch_project_graph(workspace_dir: &Path) -> Result<Value> {
    let (cmd, mut base_args) = nx_command();
    base_args.extend(["graph", "--file=stdout"]);

    let output = Command::new(cmd)
        .args(&base_args)
        .current_dir(workspace_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Failed to run nx graph --file=stdout in: {}",
                workspace_dir.display()
            )
        })?;

    if !output.status.success() {
        anyhow::bail!(
            "nx graph --file=stdout failed in {}",
            workspace_dir.display(),
        );
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&output_str).context("Failed to parse nx graph JSON output")
}

/// Extract targets for all projects from the graph JSON.
///
/// The graph JSON structure is:
/// ```json
/// { "graph": { "nodes": { "<project>": { "data": { "targets": { ... } } } } } }
/// ```
fn extract_targets_from_graph(graph: &Value, workspace_name: &str) -> Vec<NxTarget> {
    let mut all_targets = Vec::new();

    let nodes = match graph
        .get("graph")
        .and_then(|g| g.get("nodes"))
        .and_then(|n| n.as_object())
    {
        Some(n) => n,
        None => return all_targets,
    };

    for (project_name, node) in nodes {
        let targets_obj = match node
            .get("data")
            .and_then(|d| d.get("targets"))
            .and_then(|t| t.as_object())
        {
            Some(t) => t,
            None => continue,
        };

        let project_category = format!("nx:{}:{}", workspace_name, project_name);

        for (target_name, _target_config) in targets_obj {
            let qualified_name = format!("{}:{}", project_name, target_name);
            let display_name = format_display_name(target_name);
            let description = format!("nx run {}:{}", project_name, target_name);

            all_targets.push(NxTarget {
                name: qualified_name,
                display_name,
                category: project_category.clone(),
                description,
                emoji: Some("\u{1f537}".to_string()), // 🔷
                ignored: false,
                project: project_name.to_string(),
                target: target_name.to_string(),
            });
        }
    }

    all_targets
}

/// List all targets from all projects in an Nx workspace.
///
/// Fetches the entire project graph via `nx graph --file=stdout` in a single
/// CLI call, then extracts targets from the JSON response. This is dramatically
/// faster than querying each project individually, especially for large monorepos
/// (e.g. 85 projects in ~1s instead of ~42s).
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

    let graph = fetch_project_graph(workspace_dir)?;
    let mut all_targets = extract_targets_from_graph(&graph, category);

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

    /// Build a graph JSON value from a list of (project_name, [target_names]) pairs.
    fn build_graph_json(projects: &[(&str, &[&str])]) -> Value {
        let mut nodes = serde_json::Map::new();
        for (project, targets) in projects {
            let mut targets_obj = serde_json::Map::new();
            for target in *targets {
                targets_obj.insert(target.to_string(), serde_json::json!({}));
            }
            nodes.insert(
                project.to_string(),
                serde_json::json!({
                    "name": project,
                    "type": "app",
                    "data": {
                        "root": format!("apps/{}", project),
                        "name": project,
                        "targets": targets_obj,
                    }
                }),
            );
        }
        serde_json::json!({ "graph": { "nodes": nodes, "dependencies": {} } })
    }

    fn sample_graph_json() -> Value {
        build_graph_json(&[("my-app", &["build", "serve", "test", "lint"])])
    }

    #[test]
    fn test_extract_targets_extracts_all_targets() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        assert_eq!(targets.len(), 4);

        let names: Vec<&str> = targets.iter().map(|t| t.target.as_str()).collect();
        assert!(names.contains(&"build"));
        assert!(names.contains(&"serve"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"lint"));
    }

    #[test]
    fn test_extract_targets_qualified_names() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        let build = targets.iter().find(|t| t.target == "build").unwrap();
        assert_eq!(build.name, "my-app:build");

        let serve = targets.iter().find(|t| t.target == "serve").unwrap();
        assert_eq!(serve.name, "my-app:serve");
    }

    #[test]
    fn test_extract_targets_descriptions() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        let build = targets.iter().find(|t| t.target == "build").unwrap();
        assert_eq!(build.description, "nx run my-app:build");

        let test = targets.iter().find(|t| t.target == "test").unwrap();
        assert_eq!(test.description, "nx run my-app:test");
    }

    #[test]
    fn test_extract_targets_display_names() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        let build = targets.iter().find(|t| t.target == "build").unwrap();
        assert_eq!(build.display_name, "Build");

        let serve = targets.iter().find(|t| t.target == "serve").unwrap();
        assert_eq!(serve.display_name, "Serve");
    }

    #[test]
    fn test_extract_targets_category() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "my-workspace");

        for target in &targets {
            assert_eq!(target.category, "nx:my-workspace:my-app");
        }
    }

    #[test]
    fn test_extract_targets_emoji() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        for target in &targets {
            assert_eq!(target.emoji, Some("\u{1f537}".to_string())); // 🔷
        }
    }

    #[test]
    fn test_extract_targets_project_field() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        for target in &targets {
            assert_eq!(target.project, "my-app");
        }
    }

    #[test]
    fn test_extract_targets_empty_targets() {
        let graph = build_graph_json(&[("empty-project", &[])]);
        let targets = extract_targets_from_graph(&graph, "myworkspace");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_extract_targets_no_targets_key() {
        // Node with data but no targets key
        let graph = serde_json::json!({
            "graph": {
                "nodes": {
                    "no-targets": {
                        "name": "no-targets",
                        "type": "lib",
                        "data": {
                            "root": "libs/no-targets",
                            "name": "no-targets"
                        }
                    }
                },
                "dependencies": {}
            }
        });
        let targets = extract_targets_from_graph(&graph, "myworkspace");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_extract_targets_empty_graph() {
        let graph = serde_json::json!({ "graph": { "nodes": {}, "dependencies": {} } });
        let targets = extract_targets_from_graph(&graph, "myworkspace");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_extract_targets_missing_graph_key() {
        let graph = serde_json::json!({});
        let targets = extract_targets_from_graph(&graph, "myworkspace");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_extract_targets_complex_target_names() {
        let graph = build_graph_json(&[(
            "my-lib",
            &["build", "build-storybook", "component-test", "e2e"],
        )]);
        let targets = extract_targets_from_graph(&graph, "myworkspace");

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
    fn test_extract_targets_multiple_projects() {
        let graph = build_graph_json(&[("app", &["build", "serve"]), ("lib", &["build", "test"])]);
        let targets = extract_targets_from_graph(&graph, "ws");

        let app_targets: Vec<_> = targets.iter().filter(|t| t.project == "app").collect();
        let lib_targets: Vec<_> = targets.iter().filter(|t| t.project == "lib").collect();

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
    fn test_extract_targets_per_project_categories() {
        let graph = build_graph_json(&[
            ("app", &["build", "test", "lint"]),
            ("lib", &["build", "test", "lint"]),
        ]);
        let targets = extract_targets_from_graph(&graph, "monopoly");

        let app_targets: Vec<_> = targets.iter().filter(|t| t.project == "app").collect();
        let lib_targets: Vec<_> = targets.iter().filter(|t| t.project == "lib").collect();

        // Even though both have the same target names, they should have different categories
        assert_ne!(app_targets[0].category, lib_targets[0].category);
        assert_eq!(app_targets[0].category, "nx:monopoly:app");
        assert_eq!(lib_targets[0].category, "nx:monopoly:lib");
    }

    #[test]
    fn test_extract_targets_not_ignored() {
        let graph = sample_graph_json();
        let targets = extract_targets_from_graph(&graph, "myworkspace");

        for target in &targets {
            assert!(!target.ignored);
        }
    }

    #[test]
    fn test_collect_category_display_names() {
        let graph = build_graph_json(&[("service.auth", &["build"]), ("service.api", &["test"])]);
        let all_targets = extract_targets_from_graph(&graph, "monopoly");

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
