//! Parser for Task (go-task/Taskfile) - uses `task --list-all --json` output.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::script::discovery::format_display_name;

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
}

/// Parse JSON output from `task --list-all --json` into a list of TaskTask.
pub fn parse_task_list_json(json_str: &str, category: &str) -> Result<Vec<TaskTask>> {
    let output: TaskListOutput = serde_json::from_str(json_str)
        .with_context(|| "Failed to parse task --list-all --json output")?;

    let mut tasks = Vec::new();
    for info in output.tasks {
        let display_name = format_display_name(&info.name);
        let description = info
            .desc
            .or(info.summary)
            .unwrap_or_else(|| format!("task {}", info.name));

        tasks.push(TaskTask {
            name: info.name,
            display_name,
            category: category.to_string(),
            description,
        });
    }

    tasks.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tasks)
}

/// Check if the `task` binary is available.
pub fn is_task_available() -> bool {
    Command::new("task")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run `task --list-all --json --taskfile <path>` and parse the result.
pub fn list_tasks(taskfile_path: &Path, category: &str) -> Result<Vec<TaskTask>> {
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

    let json_str = String::from_utf8(output.stdout)
        .with_context(|| "task output was not valid UTF-8")?;

    parse_task_list_json(json_str.trim(), category)
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

        let result = parse_task_list_json(json, "mydir").unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].name, "build");
        assert_eq!(result[0].display_name, "Build");
        assert_eq!(result[0].category, "mydir");
        assert_eq!(result[0].description, "Build the project");

        assert_eq!(result[1].name, "test");
        assert_eq!(result[1].display_name, "Test");
        assert_eq!(result[1].description, "Run tests");
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

        let result = parse_task_list_json(json, "cat").unwrap();
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

        let result = parse_task_list_json(json, "cat").unwrap();
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

        let result = parse_task_list_json(json, "cat").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "task build");
    }

    #[test]
    fn test_parse_task_list_json_empty() {
        let json = r#"{"tasks": [], "location": "/x"}"#;
        let result = parse_task_list_json(json, "cat").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_task_list_json_invalid() {
        let json = r#"{"tasks": [{"name": "x"}"#;
        assert!(parse_task_list_json(json, "cat").is_err());
    }
}
