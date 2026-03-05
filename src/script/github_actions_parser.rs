//! # GitHub Actions Parser
//!
//! This module discovers and parses GitHub Actions workflow files from
//! `.github/workflows/` directories.
//!
//! ## Overview
//!
//! GitHub Actions workflows are defined as YAML files. This parser extracts:
//! - Workflow name (from the `name:` field)
//! - Trigger events (from the `on:` field)
//! - Job names (from the `jobs:` section)
//!
//! The parser uses regex-based YAML extraction to avoid adding a `serde_yaml`
//! dependency, consistent with the rest of the codebase.
//!
//! ## Key Types
//!
//! - [`GithubWorkflow`] — Represents a single workflow file with display metadata
//! - [`is_gh_available`] — Checks if the `gh` CLI is installed
//! - [`is_act_available`] — Checks if the `act` CLI is installed (nektos/act)
//! - [`list_workflows`] — Main function to list workflows from `.github/workflows/`
//!
//! ## Execution
//!
//! Workflows are **read-only** by default (informational display).
//! If `act` is installed, the workflow is run locally via Docker using
//! `act -W .github/workflows/<filename>`. Otherwise, if `gh` CLI is available,
//! `gh workflow run <filename>` is used to trigger the workflow remotely.
//!
//! ## Availability Caching
//!
//! The `gh` and `act` binary availability are each cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::script::discovery::format_display_name;

/// Cache for act CLI availability check (checked once per process)
static ACT_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if the `act` CLI is available (<https://github.com/nektos/act>).
///
/// `act` allows running GitHub Actions workflows locally using Docker.
pub fn is_act_available() -> bool {
    *ACT_AVAILABLE.get_or_init(|| {
        Command::new("act")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Cache for gh CLI availability check (checked once per process)
static GH_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if the `gh` CLI is available.
pub fn is_gh_available() -> bool {
    *GH_AVAILABLE.get_or_init(|| {
        Command::new("gh")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Select the best trigger event to pass to `act` for a given workflow file.
///
/// Event priority: `workflow_dispatch` → `push` → `pull_request` → first trigger → `None`.
/// Returning `None` means letting `act` pick its own default.
pub fn select_act_event(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let triggers = extract_triggers(&content);
    if triggers.is_empty() {
        return None;
    }
    const PRIORITY: &[&str] = &["workflow_dispatch", "push", "pull_request", "schedule"];
    for event in PRIORITY {
        if triggers.iter().any(|t| t == *event) {
            return Some((*event).to_string());
        }
    }
    triggers.into_iter().next()
}

/// A parsed GitHub Actions workflow item for TUI display.
#[derive(Debug, Clone)]
pub struct GithubWorkflow {
    /// The bare filename, e.g. `build.yml`
    pub file_name: String,
    /// The `name:` field from the workflow YAML, or the filename stem if absent
    pub workflow_name: String,
    /// Formatted display name shown in the TUI
    pub display_name: String,
    /// Category string (always `"GitHub Actions"`)
    pub category: String,
    /// Job IDs extracted from the `jobs:` section
    pub jobs: Vec<String>,
    /// Trigger event names extracted from the `on:` field
    pub triggers: Vec<String>,
    /// Human-readable summary: triggers and jobs
    pub description: String,
    /// Hidden from TUI (always false for workflows)
    pub ignored: bool,
}

/// Parse a single workflow YAML file and return a [`GithubWorkflow`].
pub fn parse_workflow_file(path: &Path, category: &str) -> Result<GithubWorkflow> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read workflow file: {}", path.display()))?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| format!("Invalid filename: {}", path.display()))?
        .to_string();

    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&file_name)
        .to_string();

    // Extract the top-level `name:` field (not indented)
    let workflow_name = extract_workflow_name(&content).unwrap_or(file_stem);

    // Extract trigger events from the `on:` block
    let triggers = extract_triggers(&content);

    // Extract job IDs from the `jobs:` block
    let jobs = extract_jobs(&content);

    let display_name = format!("🐙 {}", format_display_name(&workflow_name));

    // Build a compact description
    let description = build_description(&triggers, &jobs);

    Ok(GithubWorkflow {
        file_name,
        workflow_name,
        display_name,
        category: category.to_string(),
        jobs,
        triggers,
        description,
        ignored: false,
    })
}

/// Extract the top-level `name:` value from workflow YAML content.
fn extract_workflow_name(content: &str) -> Option<String> {
    for line in content.lines() {
        // Match `name: <value>` at column 0 (not indented)
        if let Some(stripped) = line.strip_prefix("name:") {
            let value = stripped.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Extract trigger event names from the `on:` section of workflow YAML.
///
/// Handles several YAML shapes:
/// - `on: push`  (single, inline)
/// - `on: [push, pull_request]`  (inline list)
/// - `on:\n  push:\n  pull_request:` (block mapping)
fn extract_triggers(content: &str) -> Vec<String> {
    let mut triggers = Vec::new();
    let mut in_on_block = false;
    let mut block_indent: Option<usize> = None;

    for line in content.lines() {
        // Detect the top-level `on:` key
        if let Some(after_on) = line.strip_prefix("on:") {
            in_on_block = true;
            block_indent = None;
            let inline = after_on.trim();

            if inline.is_empty() {
                // Block form — will be collected on subsequent lines
                continue;
            }

            // Inline list form: `on: [push, pull_request]`
            if inline.starts_with('[') {
                let inner = inline.trim_start_matches('[').trim_end_matches(']');
                for part in inner.split(',') {
                    let t = part.trim().trim_matches('"').trim_matches('\'');
                    if !t.is_empty() {
                        triggers.push(t.to_string());
                    }
                }
                in_on_block = false;
                continue;
            }

            // Inline single value: `on: push`
            let t = inline.trim_matches('"').trim_matches('\'');
            if !t.is_empty() {
                triggers.push(t.to_string());
            }
            in_on_block = false;
            continue;
        }

        if !in_on_block {
            continue;
        }

        // Any top-level key (no indent) ends the `on:` block
        if !line.starts_with(' ') && !line.starts_with('\t') && !line.trim().is_empty() {
            in_on_block = false;
            block_indent = None;
            continue;
        }

        // Detect the indent level from the first indented line inside the `on:` block
        let leading = line.len() - line.trim_start().len();
        if leading == 0 {
            continue;
        }
        let indent = block_indent.get_or_insert(leading);
        if leading == *indent {
            let key = line.trim_start().trim_end_matches(':').trim();
            if !key.is_empty() && !key.starts_with('#') {
                triggers.push(key.to_string());
            }
        }
    }

    triggers
}

/// Extract job IDs from the `jobs:` section of workflow YAML.
fn extract_jobs(content: &str) -> Vec<String> {
    let mut jobs = Vec::new();
    let mut in_jobs_block = false;

    for line in content.lines() {
        if line.starts_with("jobs:") {
            in_jobs_block = true;
            continue;
        }

        if !in_jobs_block {
            continue;
        }

        // Job IDs are at exactly 2-space indent
        if line.starts_with("  ") && !line.starts_with("   ") {
            let key = line.trim_start().trim_end_matches(':').trim();
            if !key.is_empty() && !key.starts_with('#') {
                jobs.push(key.to_string());
            }
            continue;
        }

        // Any non-indented, non-empty line ends the jobs block
        if !line.starts_with(' ') && !line.starts_with('\t') && !line.trim().is_empty() {
            in_jobs_block = false;
        }
    }

    jobs
}

/// Build a human-readable description from triggers and jobs.
fn build_description(triggers: &[String], jobs: &[String]) -> String {
    let mut parts = Vec::new();

    if !triggers.is_empty() {
        parts.push(format!("on: {}", triggers.join(", ")));
    }

    if !jobs.is_empty() {
        parts.push(format!("jobs: {}", jobs.join(", ")));
    }

    if parts.is_empty() {
        "GitHub Actions workflow".to_string()
    } else {
        parts.join(" | ")
    }
}

/// Discover all workflow files under `workflows_dir` (`.github/workflows/`)
/// and return a list of [`GithubWorkflow`] items.
pub fn list_workflows(workflows_dir: &Path, category: &str) -> Result<Vec<GithubWorkflow>> {
    let mut workflows = Vec::new();

    if !workflows_dir.exists() {
        return Ok(workflows);
    }

    let mut entries: Vec<_> = std::fs::read_dir(workflows_dir)
        .with_context(|| {
            format!(
                "Failed to read workflows directory: {}",
                workflows_dir.display()
            )
        })?
        .filter_map(Result::ok)
        .filter(|e| {
            let p = e.path();
            p.is_file()
                && matches!(
                    p.extension().and_then(|x| x.to_str()),
                    Some("yml") | Some("yaml")
                )
        })
        .collect();

    // Sort by filename for deterministic ordering
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        match parse_workflow_file(&path, category) {
            Ok(workflow) => workflows.push(workflow),
            Err(e) => return Err(e),
        }
    }

    Ok(workflows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_workflow(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(filename);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_extract_workflow_name_present() {
        let content = "name: CI Build\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        assert_eq!(extract_workflow_name(content), Some("CI Build".to_string()));
    }

    #[test]
    fn test_extract_workflow_name_quoted() {
        let content = "name: \"My Workflow\"\non: push\n";
        assert_eq!(
            extract_workflow_name(content),
            Some("My Workflow".to_string())
        );
    }

    #[test]
    fn test_extract_workflow_name_absent() {
        let content = "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        assert_eq!(extract_workflow_name(content), None);
    }

    #[test]
    fn test_extract_triggers_inline_single() {
        let content = "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let triggers = extract_triggers(content);
        assert_eq!(triggers, vec!["push"]);
    }

    #[test]
    fn test_extract_triggers_inline_list() {
        let content = "on: [push, pull_request]\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let triggers = extract_triggers(content);
        assert_eq!(triggers, vec!["push", "pull_request"]);
    }

    #[test]
    fn test_extract_triggers_block_form() {
        let content =
            "on:\n  push:\n    branches: [main]\n  pull_request:\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let triggers = extract_triggers(content);
        assert_eq!(triggers, vec!["push", "pull_request"]);
    }

    #[test]
    fn test_extract_triggers_workflow_dispatch() {
        let content =
            "on:\n  workflow_dispatch:\n  push:\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let triggers = extract_triggers(content);
        assert!(triggers.contains(&"workflow_dispatch".to_string()));
        assert!(triggers.contains(&"push".to_string()));
    }

    #[test]
    fn test_extract_jobs() {
        let content =
            "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n  test:\n    runs-on: ubuntu-latest\n";
        let jobs = extract_jobs(content);
        assert_eq!(jobs, vec!["build", "test"]);
    }

    #[test]
    fn test_extract_jobs_empty() {
        let content = "on: push\n";
        let jobs = extract_jobs(content);
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_build_description() {
        let triggers = vec!["push".to_string(), "pull_request".to_string()];
        let jobs = vec!["build".to_string(), "test".to_string()];
        let desc = build_description(&triggers, &jobs);
        assert!(desc.contains("push"));
        assert!(desc.contains("build"));
    }

    #[test]
    fn test_build_description_empty() {
        let desc = build_description(&[], &[]);
        assert_eq!(desc, "GitHub Actions workflow");
    }

    #[test]
    fn test_parse_workflow_file() {
        let temp_dir = TempDir::new().unwrap();
        let content = "name: CI\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n";
        let path = write_workflow(temp_dir.path(), "ci.yml", content);

        let workflow = parse_workflow_file(&path, "GitHub Actions").unwrap();

        assert_eq!(workflow.file_name, "ci.yml");
        assert_eq!(workflow.workflow_name, "CI");
        assert!(workflow.display_name.contains("🐙"));
        assert_eq!(workflow.category, "GitHub Actions");
        assert_eq!(workflow.triggers, vec!["push"]);
        assert_eq!(workflow.jobs, vec!["build"]);
        assert!(!workflow.ignored);
    }

    #[test]
    fn test_parse_workflow_file_no_name() {
        let temp_dir = TempDir::new().unwrap();
        let content = "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n";
        let path = write_workflow(temp_dir.path(), "build.yml", content);

        let workflow = parse_workflow_file(&path, "GitHub Actions").unwrap();

        // Should fall back to file stem
        assert_eq!(workflow.workflow_name, "build");
        assert_eq!(workflow.file_name, "build.yml");
    }

    #[test]
    fn test_list_workflows_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let result = list_workflows(temp_dir.path(), "GitHub Actions").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_workflows_nonexistent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");
        let result = list_workflows(&nonexistent, "GitHub Actions").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_workflows_multiple_files() {
        let temp_dir = TempDir::new().unwrap();

        write_workflow(
            temp_dir.path(),
            "ci.yml",
            "name: CI\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n",
        );
        write_workflow(
            temp_dir.path(),
            "release.yaml",
            "name: Release\non: [push, workflow_dispatch]\njobs:\n  release:\n    runs-on: ubuntu-latest\n",
        );

        let workflows = list_workflows(temp_dir.path(), "GitHub Actions").unwrap();
        assert_eq!(workflows.len(), 2);

        // Should be sorted by filename
        assert_eq!(workflows[0].file_name, "ci.yml");
        assert_eq!(workflows[1].file_name, "release.yaml");
    }

    #[test]
    fn test_list_workflows_ignores_non_yaml() {
        let temp_dir = TempDir::new().unwrap();

        write_workflow(
            temp_dir.path(),
            "ci.yml",
            "name: CI\non: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n",
        );
        fs::write(temp_dir.path().join("readme.txt"), "not a workflow").unwrap();
        fs::write(temp_dir.path().join("config.json"), "{}").unwrap();

        let workflows = list_workflows(temp_dir.path(), "GitHub Actions").unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].file_name, "ci.yml");
    }

    #[test]
    fn test_select_act_event_workflow_dispatch_preferred() {
        let temp_dir = TempDir::new().unwrap();
        let path = write_workflow(
            temp_dir.path(),
            "deploy.yml",
            "on:\n  push:\n  workflow_dispatch:\njobs:\n  deploy:\n    runs-on: ubuntu-latest\n",
        );
        // workflow_dispatch should win over push
        assert_eq!(
            select_act_event(&path),
            Some("workflow_dispatch".to_string())
        );
    }

    #[test]
    fn test_select_act_event_push_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let path = write_workflow(
            temp_dir.path(),
            "ci.yml",
            "on:\n  push:\n  pull_request:\njobs:\n  build:\n    runs-on: ubuntu-latest\n",
        );
        // push wins over pull_request
        assert_eq!(select_act_event(&path), Some("push".to_string()));
    }

    #[test]
    fn test_select_act_event_pull_request_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let path = write_workflow(
            temp_dir.path(),
            "pr.yml",
            "on:\n  pull_request:\njobs:\n  check:\n    runs-on: ubuntu-latest\n",
        );
        assert_eq!(select_act_event(&path), Some("pull_request".to_string()));
    }

    #[test]
    fn test_select_act_event_inline_single_trigger() {
        let temp_dir = TempDir::new().unwrap();
        let path = write_workflow(
            temp_dir.path(),
            "simple.yml",
            "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n",
        );
        assert_eq!(select_act_event(&path), Some("push".to_string()));
    }

    #[test]
    fn test_select_act_event_no_triggers_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let path = write_workflow(
            temp_dir.path(),
            "empty.yml",
            "jobs:\n  build:\n    runs-on: ubuntu-latest\n",
        );
        assert_eq!(select_act_event(&path), None);
    }

    #[test]
    fn test_select_act_event_nonexistent_file_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.yml");
        assert_eq!(select_act_event(&path), None);
    }
}
