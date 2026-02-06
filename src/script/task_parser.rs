//! # Task (go-task) Parser
//!
//! This module parses Taskfile configurations using the `task` CLI's JSON output
//! for display in the Jarvis TUI.
//!
//! ## Overview
//!
//! Unlike other parsers that read configuration files directly, this parser
//! invokes `task --list-all --json` to get task information. This approach:
//!
//! - Handles complex Taskfile includes and imports
//! - Respects task visibility and internal tasks
//! - Gets accurate task descriptions from the CLI
//!
//! ## Key Types
//!
//! - [`TaskListOutput`] - JSON output from `task --list-all --json`
//! - [`TaskInfo`] - Single task metadata from the CLI
//! - [`TaskTask`] - Represents a task with display metadata for the TUI
//! - [`is_task_available`] - Checks if `task` CLI is installed
//! - [`list_tasks`] - Main function to list tasks from a Taskfile
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! task --list-all --json --taskfile <path>
//! ```
//!
//! And parses the JSON output which includes:
//! - Task name and description
//! - Task summary (fallback for description)
//! - Location information (file, line, column)
//!
//! ## Availability Caching
//!
//! The `task` binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::script::discovery::format_display_name;

/// Cache for task availability check (checked once per process)
static TASK_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// JSON output from `task --list-all --json --taskfile <path>`
#[derive(Debug, Clone, Deserialize)]
pub struct TaskListOutput {
    pub tasks: Vec<TaskInfo>,
    #[allow(dead_code)]
    pub location: Option<String>,
}

/// Single task in the Task list JSON
#[derive(Debug, Clone, Deserialize)]
pub struct TaskInfo {
    pub name: String,
    pub desc: Option<String>,
    pub summary: Option<String>,
    #[allow(dead_code)]
    pub up_to_date: Option<bool>,
    #[allow(dead_code)]
    pub location: Option<TaskLocation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskLocation {
    #[allow(dead_code)]
    pub line: Option<u64>,
    #[allow(dead_code)]
    pub column: Option<u64>,
    #[allow(dead_code)]
    pub taskfile: Option<String>,
}

/// Task item for TUI display (mirrors NpmScript/DevboxScript)
#[derive(Debug, Clone)]
pub struct TaskTask {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
}

/// Annotations extracted from YAML comments above a task definition
#[derive(Debug, Clone, Default)]
pub struct TaskAnnotations {
    pub emoji: Option<String>,
    pub description: Option<String>,
    pub ignored: bool,
}

/// Parse annotations from YAML comments in a Taskfile.
///
/// Looks for special comment annotations above task definitions:
/// - `# @emoji <emoji>` - Display emoji prefix in the TUI
/// - `# @description <text>` - Custom description for the details panel
/// - `# @ignore` - Hide the task from the TUI
///
/// Returns a map of task names to their annotations.
pub fn parse_taskfile_annotations(
    taskfile_path: &Path,
) -> Result<HashMap<String, TaskAnnotations>> {
    let content = fs::read_to_string(taskfile_path)
        .with_context(|| format!("Failed to read Taskfile: {}", taskfile_path.display()))?;

    parse_taskfile_annotations_from_content(&content)
}

/// Parse annotations from Taskfile content (for testing).
pub fn parse_taskfile_annotations_from_content(
    content: &str,
) -> Result<HashMap<String, TaskAnnotations>> {
    let mut annotations_map: HashMap<String, TaskAnnotations> = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();

    // Regex patterns for annotations (same as bash parser)
    let emoji_re =
        Regex::new(r"^\s*#\s*@emoji\s+(.+)$").context("Failed to compile emoji regex pattern")?;
    let desc_re = Regex::new(r"^\s*#\s*@description\s+(.+)$")
        .context("Failed to compile description regex pattern")?;
    let ignore_re =
        Regex::new(r"^\s*#\s*@ignore\s*$").context("Failed to compile ignore regex pattern")?;
    let comment_re = Regex::new(r"^\s*#").context("Failed to compile comment regex pattern")?;

    // Regex to match task definitions in YAML
    // Matches: "  task_name:" at the beginning of a line (with optional leading whitespace)
    // Must be under 'tasks:' section, so we track that
    let task_def_re = Regex::new(r"^(\s*)([a-zA-Z_][a-zA-Z0-9_-]*):\s*$")
        .context("Failed to compile task definition regex")?;
    let tasks_section_re =
        Regex::new(r"^tasks:\s*$").context("Failed to compile tasks section regex")?;

    let mut in_tasks_section = false;
    let mut tasks_indent: Option<usize> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        // Check if we're entering the tasks: section
        if tasks_section_re.is_match(line) {
            in_tasks_section = true;
            tasks_indent = None;
            continue;
        }

        // If we're in the tasks section, look for task definitions
        if in_tasks_section {
            // Check for end of tasks section (another top-level key)
            if !line.trim().is_empty()
                && !line.starts_with(' ')
                && !line.starts_with('\t')
                && !comment_re.is_match(line)
            {
                in_tasks_section = false;
                tasks_indent = None;
                continue;
            }

            // Match task definitions
            if let Some(cap) = task_def_re.captures(line) {
                let indent = cap[1].len();
                let task_name = &cap[2];

                // Determine or verify the task indent level
                match tasks_indent {
                    None => {
                        // First task we see - this is the task indent level
                        tasks_indent = Some(indent);
                    }
                    Some(expected) => {
                        // Skip if this isn't at the right indent level (could be a nested key)
                        if indent != expected {
                            continue;
                        }
                    }
                }

                // Extract annotations from preceding comment lines
                let mut emoji: Option<String> = None;
                let mut description: Option<String> = None;
                let mut ignored = false;

                // Look backwards from the task line through consecutive comment lines
                let mut check_idx = line_idx.saturating_sub(1);
                loop {
                    if check_idx >= line_idx {
                        break; // Underflow protection
                    }

                    let prev_line = lines[check_idx];

                    // If we hit a non-comment, non-empty line, stop looking back
                    if !prev_line.trim().is_empty() && !comment_re.is_match(prev_line) {
                        break;
                    }

                    // Check for ignore annotation
                    if ignore_re.is_match(prev_line) {
                        ignored = true;
                    }

                    // Check for emoji annotation
                    if let Some(emoji_cap) = emoji_re.captures(prev_line) {
                        emoji = Some(emoji_cap[1].trim().to_string());
                    }

                    // Check for description annotation
                    if let Some(desc_cap) = desc_re.captures(prev_line) {
                        description = Some(desc_cap[1].trim().to_string());
                    }

                    if check_idx == 0 {
                        break;
                    }
                    check_idx -= 1;
                }

                // Only add if there are any annotations
                if emoji.is_some() || description.is_some() || ignored {
                    annotations_map.insert(
                        task_name.to_string(),
                        TaskAnnotations {
                            emoji,
                            description,
                            ignored,
                        },
                    );
                }
            }
        }
    }

    Ok(annotations_map)
}

/// Parse JSON output from `task --list-all --json` into a list of TaskTask.
///
/// If annotations are provided, they will be applied to matching tasks.
pub fn parse_task_list_json(
    json_str: &str,
    category: &str,
    annotations: Option<&HashMap<String, TaskAnnotations>>,
) -> Result<Vec<TaskTask>> {
    let output: TaskListOutput = serde_json::from_str(json_str)
        .with_context(|| "Failed to parse task --list-all --json output")?;

    let mut tasks = Vec::new();
    for info in output.tasks {
        let display_name = format_display_name(&info.name);

        // Look up annotations for this task
        let task_annotations = annotations.and_then(|a| a.get(&info.name));

        // Use annotation description if available, otherwise fall back to task desc/summary
        let description = task_annotations
            .and_then(|a| a.description.clone())
            .or(info.desc)
            .or(info.summary)
            .unwrap_or_else(|| format!("task {}", info.name));

        let emoji = task_annotations.and_then(|a| a.emoji.clone());
        let ignored = task_annotations.is_some_and(|a| a.ignored);

        tasks.push(TaskTask {
            name: info.name,
            display_name,
            category: category.to_string(),
            description,
            emoji,
            ignored,
        });
    }

    tasks.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tasks)
}

/// Check if the `task` binary is available.
pub fn is_task_available() -> bool {
    *TASK_AVAILABLE.get_or_init(|| {
        Command::new("task")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Run `task --list-all --json --taskfile <path>` and parse the result.
///
/// Also parses annotations from the Taskfile.yml comments.
pub fn list_tasks(taskfile_path: &Path, category: &str) -> Result<Vec<TaskTask>> {
    // First, parse annotations from the Taskfile
    let annotations = parse_taskfile_annotations(taskfile_path).ok();

    let output = Command::new("task")
        .arg("--list-all")
        .arg("--json")
        .arg("--taskfile")
        .arg(taskfile_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("Failed to run task for: {}", taskfile_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "task --list-all failed for {}: {}",
            taskfile_path.display(),
            stderr,
        );
    }

    let output_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).to_string(),
    };
    parse_task_list_json(output_str.trim(), category, annotations.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_task_list_json_valid() {
        let json = r#"{
            "tasks": [
                {
                    "name": "build",
                    "desc": "Build the project",
                    "summary": "",
                    "up_to_date": false
                },
                {
                    "name": "test",
                    "desc": "Run tests",
                    "summary": "",
                    "up_to_date": false
                }
            ],
            "location": "/path/to/Taskfile.yml"
        }"#;

        let result = parse_task_list_json(json, "mydir", None).unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].name, "build");
        assert_eq!(result[0].display_name, "Build");
        assert_eq!(result[0].category, "mydir");
        assert_eq!(result[0].description, "Build the project");
        assert_eq!(result[0].emoji, None);
        assert!(!result[0].ignored);

        assert_eq!(result[1].name, "test");
        assert_eq!(result[1].display_name, "Test");
        assert_eq!(result[1].description, "Run tests");
        assert_eq!(result[1].emoji, None);
        assert!(!result[1].ignored);
    }

    #[test]
    fn test_parse_task_list_json_sorted() {
        let json = r#"{
            "tasks": [
                {"name": "z_last", "desc": "Last"},
                {"name": "a_first", "desc": "First"}
            ],
            "location": "/x"
        }"#;

        let result = parse_task_list_json(json, "cat", None).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "a_first");
        assert_eq!(result[1].name, "z_last");
    }

    #[test]
    fn test_parse_task_list_json_no_desc_uses_summary() {
        let json = r#"{
            "tasks": [
                {"name": "build", "summary": "Build summary"}
            ],
            "location": "/x"
        }"#;

        let result = parse_task_list_json(json, "cat", None).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Build summary");
    }

    #[test]
    fn test_parse_task_list_json_no_desc_no_summary() {
        let json = r#"{
            "tasks": [
                {"name": "build"}
            ],
            "location": "/x"
        }"#;

        let result = parse_task_list_json(json, "cat", None).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "task build");
    }

    #[test]
    fn test_parse_task_list_json_empty() {
        let json = r#"{"tasks": [], "location": "/x"}"#;
        let result = parse_task_list_json(json, "cat", None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_task_list_json_invalid() {
        let json = r#"{"tasks": [{"name": "x"}"#;
        assert!(parse_task_list_json(json, "cat", None).is_err());
    }

    // Tests for annotation parsing

    #[test]
    fn test_parse_taskfile_annotations_emoji() {
        let content = r#"version: '3'

tasks:
  # @emoji 🚀
  deploy:
    desc: Deploy the application
    cmds:
      - echo "Deploying..."
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy"));
        assert_eq!(annotations["deploy"].emoji, Some("🚀".to_string()));
        assert!(!annotations["deploy"].ignored);
    }

    #[test]
    fn test_parse_taskfile_annotations_description() {
        let content = r#"version: '3'

tasks:
  # @description Custom description for build
  build:
    cmds:
      - echo "Building..."
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("build"));
        assert_eq!(
            annotations["build"].description,
            Some("Custom description for build".to_string())
        );
    }

    #[test]
    fn test_parse_taskfile_annotations_ignore() {
        let content = r#"version: '3'

tasks:
  # @ignore
  _internal_helper:
    cmds:
      - echo "Internal helper"
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("_internal_helper"));
        assert!(annotations["_internal_helper"].ignored);
    }

    #[test]
    fn test_parse_taskfile_annotations_multiple() {
        let content = r#"version: '3'

tasks:
  # @emoji 🔧
  # @description Build the project with optimizations
  build:
    cmds:
      - cargo build --release

  # @ignore
  _helper:
    cmds:
      - echo "helper"

  # @emoji 🧪
  test:
    desc: Run tests
    cmds:
      - cargo test
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();

        assert!(annotations.contains_key("build"));
        assert_eq!(annotations["build"].emoji, Some("🔧".to_string()));
        assert_eq!(
            annotations["build"].description,
            Some("Build the project with optimizations".to_string())
        );
        assert!(!annotations["build"].ignored);

        assert!(annotations.contains_key("_helper"));
        assert!(annotations["_helper"].ignored);

        assert!(annotations.contains_key("test"));
        assert_eq!(annotations["test"].emoji, Some("🧪".to_string()));
        assert!(!annotations["test"].ignored);
    }

    #[test]
    fn test_parse_taskfile_annotations_no_annotations() {
        let content = r#"version: '3'

tasks:
  build:
    cmds:
      - cargo build
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();
        assert!(!annotations.contains_key("build"));
    }

    #[test]
    fn test_parse_taskfile_annotations_all_three() {
        let content = r#"version: '3'

tasks:
  # @emoji 🚀
  # @description Deploy to production
  # @ignore
  deploy_internal:
    cmds:
      - ./deploy.sh
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy_internal"));
        assert_eq!(annotations["deploy_internal"].emoji, Some("🚀".to_string()));
        assert_eq!(
            annotations["deploy_internal"].description,
            Some("Deploy to production".to_string())
        );
        assert!(annotations["deploy_internal"].ignored);
    }

    #[test]
    fn test_parse_task_list_json_with_annotations() {
        let json = r#"{
            "tasks": [
                {"name": "build", "desc": "Build the project"},
                {"name": "deploy", "desc": "Deploy the app"}
            ],
            "location": "/x"
        }"#;

        let mut annotations = HashMap::new();
        annotations.insert(
            "deploy".to_string(),
            TaskAnnotations {
                emoji: Some("🚀".to_string()),
                description: Some("Custom deploy description".to_string()),
                ignored: false,
            },
        );

        let result = parse_task_list_json(json, "cat", Some(&annotations)).unwrap();
        assert_eq!(result.len(), 2);

        // build should have no annotations
        assert_eq!(result[0].name, "build");
        assert_eq!(result[0].emoji, None);
        assert_eq!(result[0].description, "Build the project");
        assert!(!result[0].ignored);

        // deploy should have annotations applied
        assert_eq!(result[1].name, "deploy");
        assert_eq!(result[1].emoji, Some("🚀".to_string()));
        assert_eq!(result[1].description, "Custom deploy description");
        assert!(!result[1].ignored);
    }

    #[test]
    fn test_parse_task_list_json_annotation_description_overrides_task_desc() {
        let json = r#"{
            "tasks": [
                {"name": "build", "desc": "Original description from task"}
            ],
            "location": "/x"
        }"#;

        let mut annotations = HashMap::new();
        annotations.insert(
            "build".to_string(),
            TaskAnnotations {
                emoji: None,
                description: Some("Overridden description from annotation".to_string()),
                ignored: false,
            },
        );

        let result = parse_task_list_json(json, "cat", Some(&annotations)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].description,
            "Overridden description from annotation"
        );
    }

    #[test]
    fn test_parse_taskfile_annotations_non_consecutive_comments() {
        // Comments that are not directly above the task should not be picked up
        let content = r#"version: '3'

tasks:
  # @emoji 🚀
  
  deploy:
    cmds:
      - echo "Deploying..."
"#;

        let annotations = parse_taskfile_annotations_from_content(content).unwrap();
        // Empty line between comment and task means annotation should still be found
        // (we look through empty lines)
        assert!(annotations.contains_key("deploy"));
    }
}
