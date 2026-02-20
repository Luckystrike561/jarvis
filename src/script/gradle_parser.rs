//! # Gradle Parser
//!
//! This module parses Gradle projects and extracts available tasks.
//!
//! ## Overview
//!
//! Gradle is a powerful build automation tool used for Java, Kotlin, Android, and polyglot projects.
//! This parser invokes `gradle tasks --all` to get task information.
//!
//! ## Key Types
//!
//! - [`GradleTask`] - Represents a Gradle task with display metadata for the TUI
//! - [`is_gradle_available`] - Checks if `gradle` CLI or Gradle wrapper is available
//! - [`list_tasks`] - Main function to list tasks from a Gradle project
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! ./gradlew tasks --all -q
//! # or
//! gradle tasks --all -q
//! ```
//!
//! And parses the output to extract tasks grouped by category.
//!
//! ## Execution
//!
//! Tasks are executed using:
//! ```bash
//! ./gradlew <task-name>
//! # or
//! gradle <task-name>
//! ```
//!
//! ## Availability Caching
//!
//! The Gradle availability is cached using [`OnceLock`] to avoid repeated process spawning.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::script::discovery::format_display_name;

static GRADLE_AVAILABLE: OnceLock<bool> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct GradleTask {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub group: Option<String>,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
}

fn get_gradle_wrapper(project_dir: &Path) -> Option<std::path::PathBuf> {
    let wrapper = project_dir.join("gradlew");
    if wrapper.exists() {
        return Some(wrapper);
    }
    let wrapper_bat = project_dir.join("gradlew.bat");
    if wrapper_bat.exists() {
        return Some(wrapper_bat);
    }
    None
}

pub fn is_gradle_available() -> bool {
    *GRADLE_AVAILABLE.get_or_init(|| {
        if Command::new("gradle")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
        false
    })
}

pub fn get_gradle_command(project_dir: &Path) -> Option<String> {
    if let Some(wrapper) = get_gradle_wrapper(project_dir) {
        return Some(wrapper.to_string_lossy().to_string());
    }

    if Command::new("gradle")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("gradle".to_string());
    }

    None
}

fn parse_gradle_tasks_output(output: &str, category: &str) -> Result<Vec<GradleTask>> {
    let mut tasks = Vec::new();
    let mut current_group: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.starts_with("------------------------------------------------------------") {
            continue;
        }

        if line.starts_with("Tasks runnable from") {
            continue;
        }

        if line.starts_with("Other tasks") {
            break;
        }

        if line.ends_with(" tasks") && !line.contains('-') {
            current_group = Some(
                line.trim_end_matches(" tasks")
                    .trim()
                    .to_string(),
            );
            continue;
        }

        if line.starts_with("Build tasks") {
            current_group = Some("Build".to_string());
            continue;
        }

        if line.starts_with('-') || line.starts_with("Other") {
            continue;
        }

        if let Some(dash_pos) = line.find(" - ") {
            let name = line[..dash_pos].trim().to_string();
            let description = line[dash_pos + 3..].trim().to_string();

            if name.is_empty() {
                continue;
            }

            let display_name = format_display_name(&name);

            let emoji = Some("\u{1F4E6}".to_string());

            tasks.push(GradleTask {
                name,
                display_name,
                category: category.to_string(),
                group: current_group.clone(),
                description,
                emoji,
                ignored: false,
            });
        }
    }

    tasks.sort_by(|a, b| {
        let group_cmp = a.group.cmp(&b.group);
        if group_cmp == std::cmp::Ordering::Equal {
            a.name.cmp(&b.name)
        } else {
            group_cmp
        }
    });

    Ok(tasks)
}

pub fn list_tasks(project_dir: &Path, category: &str) -> Result<Vec<GradleTask>> {
    let gradle_cmd = get_gradle_command(project_dir).context(
        "Neither Gradle wrapper (gradlew) nor system Gradle is available. \
         Please ensure Gradle is installed or a Gradle wrapper is present in the project.",
    )?;

    let output = if cfg!(target_os = "windows") {
        Command::new(&gradle_cmd)
            .args(["tasks", "--all", "-q"])
            .current_dir(project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .with_context(|| format!("Failed to run {} tasks", gradle_cmd))?
    } else {
        Command::new(&gradle_cmd)
            .arg("tasks")
            .arg("--all")
            .arg("-q")
            .current_dir(project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .with_context(|| format!("Failed to run {} tasks", gradle_cmd))?
    };

    if !output.status.success() {
        anyhow::bail!(
            "gradle tasks failed with exit code: {:?}",
            output.status.code()
        );
    }

    let output_str = String::from_utf8_lossy(&output.stdout).to_string();

    parse_gradle_tasks_output(&output_str, category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gradle_tasks_basic() {
        let output = r#"
------------------------------------------------------------
Tasks runnable from root project 'my-project'
------------------------------------------------------------

Build tasks
-----------
assemble - Assembles the outputs of this project.
build - Assembles and tests this project.
clean - Deletes the build directory.

Application tasks
------------------
run - Runs this project as a JVM application.

Custom tasks
-----------
myTask - My custom task description.
"#;
        let tasks = parse_gradle_tasks_output(output, "myproject").unwrap();

        assert!(tasks.len() >= 3);
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"assemble"));
        assert!(names.contains(&"build"));
        assert!(names.contains(&"clean"));
    }

    #[test]
    fn test_parse_gradle_tasks_with_groups() {
        let output = r#"
------------------------------------------------------------
Tasks runnable from root project 'my-project'
------------------------------------------------------------

Build tasks
-----------
assemble - Assembles the outputs of this project.
build - Assembles and tests this project.

Application tasks
-----------------
run - Runs this project as a JVM application.

Other tasks
-----------
wrapper - Generates Gradle wrapper files.
"#;
        let tasks = parse_gradle_tasks_output(output, "myproject").unwrap();

        let build_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| t.group.as_ref() == Some(&"Build".to_string()))
            .collect();
        assert!(!build_tasks.is_empty());

        let run_task = tasks.iter().find(|t| t.name == "run").unwrap();
        assert_eq!(run_task.group, Some("Application".to_string()));
    }

    #[test]
    fn test_parse_gradle_tasks_descriptions() {
        let output = r#"
Build tasks
-----------
assemble - Assembles the outputs of this project.
build - Assembles and tests this project.
"#;
        let tasks = parse_gradle_tasks_output(output, "myproject").unwrap();

        let assemble = tasks.iter().find(|t| t.name == "assemble").unwrap();
        assert_eq!(assemble.description, "Assembles the outputs of this project.");
    }

    #[test]
    fn test_parse_gradle_tasks_empty_output() {
        let tasks = parse_gradle_tasks_output("", "myproject").unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_gradle_tasks_category() {
        let output = "Build tasks\n-----------\nassemble - Build project.\n";
        let tasks = parse_gradle_tasks_output(output, "my-gradle-project").unwrap();

        for task in &tasks {
            assert_eq!(task.category, "my-gradle-project");
        }
    }

    #[test]
    fn test_parse_gradle_tasks_display_names() {
        let output = "Build tasks\n-----------\nassemble - Assembles the outputs of this project.\n";
        let tasks = parse_gradle_tasks_output(output, "myproject").unwrap();

        let task = tasks.iter().find(|t| t.name == "assemble").unwrap();
        assert_eq!(task.display_name, "Assemble");
    }

    #[test]
    fn test_get_gradle_wrapper_not_exists() {
        let temp_dir = std::env::temp_dir();
        let result = get_gradle_wrapper(&temp_dir);
        assert!(result.is_none());
    }
}
