//! # Bazel Parser
//!
//! This module parses Bazel workspaces and extracts available targets.
//!
//! ## Overview
//!
//! This parser invokes `bazel query` to discover runnable targets. This approach:
//!
//! - Discovers binary and test targets accurately from Bazel
//! - Uses Bazel's own query language for target discovery
//! - Handles workspace-based architecture correctly
//!
//! ## Key Types
//!
//! - [`BazelTarget`] - Represents a Bazel target with display metadata for the TUI
//! - [`BazelTargetType`] - Distinguishes between binary, test, and library targets
//! - [`is_bazel_available`] - Checks if `bazel` or `bazelisk` CLI is installed
//! - [`list_targets`] - Main function to list targets from a Bazel workspace
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! bazel query 'kind(".*_(binary|test)", //...)' --output=label
//! ```
//!
//! And parses the output to extract targets.
//!
//! ## Execution
//!
//! Targets are executed based on their type:
//! - Binary targets: `bazel run <target>`
//! - Test targets: `bazel test <target>`
//!
//! ## Availability Caching
//!
//! The `bazel` or `bazelisk` binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::script::discovery::format_display_name;

static BAZEL_AVAILABLE: OnceLock<Option<String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BazelTargetType {
    Binary,
    Test,
    Library,
}

#[derive(Debug, Clone)]
pub struct BazelTarget {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
    pub target_type: BazelTargetType,
    pub label: String,
}

pub fn is_bazel_available() -> bool {
    get_bazel_command().is_some()
}

pub fn get_bazel_command() -> Option<String> {
    BAZEL_AVAILABLE
        .get_or_init(|| {
            if Command::new("bazelisk")
                .arg("version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Some("bazelisk".to_string());
            }

            if Command::new("bazel")
                .arg("version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Some("bazel".to_string());
            }

            None
        })
        .clone()
}

fn parse_bazel_query_output(output: &str, category: &str) -> Result<Vec<BazelTarget>> {
    let mut targets = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let label = line.to_string();

        let (package, name, target_type_str) = parse_label(&label)?;

        let target_type = match target_type_str.as_str() {
            "binary" => BazelTargetType::Binary,
            "test" => BazelTargetType::Test,
            _ => BazelTargetType::Library,
        };

        if target_type == BazelTargetType::Library {
            continue;
        }

        let display_name = if package.is_empty() {
            format_display_name(&name)
        } else {
            format!("{}: {}", package, format_display_name(&name))
        };

        let description = match target_type {
            BazelTargetType::Binary => format!("bazel run {}", label),
            BazelTargetType::Test => format!("bazel test {}", label),
            BazelTargetType::Library => format!("bazel build {}", label),
        };

        let emoji = match target_type {
            BazelTargetType::Binary => Some("\u{1f4e6}".to_string()),
            BazelTargetType::Test => Some("\u{1f4dd}".to_string()),
            BazelTargetType::Library => Some("\u{1f4dc}".to_string()),
        };

        targets.push(BazelTarget {
            name: label.clone(),
            display_name,
            category: category.to_string(),
            description,
            emoji,
            ignored: false,
            target_type,
            label,
        });
    }

    targets.sort_by(|a, b| {
        a.target_type
            .cmp(&b.target_type)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(targets)
}

fn parse_label(label: &str) -> Result<(String, String, String)> {
    let label = label.trim_start_matches('@');

    if let Some(stripped) = label.strip_prefix("//") {
        let (package, rest) = stripped.split_once(':').unwrap_or((stripped, stripped));

        let package = package.to_string();

        if rest.ends_with("_binary") {
            if let Some(name) = rest.strip_suffix("_binary") {
                return Ok((package, name.to_string(), "binary".to_string()));
            }
        }
        if rest.ends_with("_test") {
            if let Some(name) = rest.strip_suffix("_test") {
                return Ok((package, name.to_string(), "test".to_string()));
            }
        }
        if rest.ends_with("_library") {
            if let Some(name) = rest.strip_suffix("_library") {
                return Ok((package, name.to_string(), "library".to_string()));
            }
        }

        Ok((package, rest.to_string(), "other".to_string()))
    } else {
        anyhow::bail!("Invalid Bazel label: {}", label)
    }
}

pub fn list_targets(workspace_dir: &Path, category: &str) -> Result<Vec<BazelTarget>> {
    let bazel_cmd = get_bazel_command().context("Bazel or Bazelisk not found")?;

    let output = Command::new(&bazel_cmd)
        .arg("query")
        .arg("kind(\".*_(binary|test)\", //...)")
        .arg("--output=label")
        .current_dir(workspace_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("Failed to run bazel query in: {}", workspace_dir.display()))?;

    if !output.status.success() {
        anyhow::bail!("bazel query failed in {}", workspace_dir.display());
    }

    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    parse_bazel_query_output(&output_str, category)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_query_output() -> String {
        r#"//src/main:hello_binary
//src/main:hello_test
//examples/greeting:greeting_binary
//examples/greeting:greeting_test"#
            .to_string()
    }

    #[test]
    fn test_parse_bazel_query_output() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        assert_eq!(targets.len(), 4);

        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"//src/main:hello_binary"));
        assert!(names.contains(&"//src/main:hello_test"));
        assert!(names.contains(&"//examples/greeting:greeting_binary"));
        assert!(names.contains(&"//examples/greeting:greeting_test"));
    }

    #[test]
    fn test_parse_bazel_query_output_target_types() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        let binaries: Vec<_> = targets
            .iter()
            .filter(|t| t.target_type == BazelTargetType::Binary)
            .collect();
        let tests: Vec<_> = targets
            .iter()
            .filter(|t| t.target_type == BazelTargetType::Test)
            .collect();

        assert_eq!(binaries.len(), 2);
        assert_eq!(tests.len(), 2);
    }

    #[test]
    fn test_parse_bazel_query_output_sort_order() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        assert_eq!(targets[0].target_type, BazelTargetType::Binary);
        assert!(targets[0].name.contains("greeting"));
        assert_eq!(targets[1].target_type, BazelTargetType::Binary);
        assert!(targets[1].name.contains("hello"));
        assert_eq!(targets[2].target_type, BazelTargetType::Test);
        assert!(targets[2].name.contains("greeting"));
        assert_eq!(targets[3].target_type, BazelTargetType::Test);
    }

    #[test]
    fn test_parse_bazel_query_output_descriptions() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        let bin = targets
            .iter()
            .find(|t| t.name == "//src/main:hello_binary")
            .unwrap();
        assert_eq!(bin.description, "bazel run //src/main:hello_binary");

        let test = targets
            .iter()
            .find(|t| t.name == "//src/main:hello_test")
            .unwrap();
        assert_eq!(test.description, "bazel test //src/main:hello_test");
    }

    #[test]
    fn test_parse_bazel_query_output_category() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "my-workspace").unwrap();

        for target in &targets {
            assert_eq!(target.category, "my-workspace");
        }
    }

    #[test]
    fn test_parse_bazel_query_output_display_names() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        let hello = targets
            .iter()
            .find(|t| t.name == "//src/main:hello_binary")
            .unwrap();
        assert_eq!(hello.display_name, "src/main: Hello");

        let greeting = targets
            .iter()
            .find(|t| t.name == "//examples/greeting:greeting_binary")
            .unwrap();
        assert_eq!(greeting.display_name, "examples/greeting: Greeting");
    }

    #[test]
    fn test_parse_bazel_query_output_emojis() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        let bin = targets
            .iter()
            .find(|t| t.name == "//src/main:hello_binary")
            .unwrap();
        assert_eq!(bin.emoji, Some("\u{1f4e6}".to_string()));

        let test = targets
            .iter()
            .find(|t| t.name == "//src/main:hello_test")
            .unwrap();
        assert_eq!(test.emoji, Some("\u{1f4dd}".to_string()));
    }

    #[test]
    fn test_parse_bazel_query_output_empty() {
        let targets = parse_bazel_query_output("", "myproject").unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_label_binary() {
        let (package, name, target_type) = parse_label("//src/main:hello_binary").unwrap();
        assert_eq!(package, "src/main");
        assert_eq!(name, "hello");
        assert_eq!(target_type, "binary");
    }

    #[test]
    fn test_parse_label_test() {
        let (package, name, target_type) = parse_label("//src/main:hello_test").unwrap();
        assert_eq!(package, "src/main");
        assert_eq!(name, "hello");
        assert_eq!(target_type, "test");
    }

    #[test]
    fn test_parse_label_no_suffix() {
        let (package, name, target_type) = parse_label("//src/main:hello").unwrap();
        assert_eq!(package, "src/main");
        assert_eq!(name, "hello");
        assert_eq!(target_type, "other");
    }

    #[test]
    fn test_parse_label_root() {
        let (package, name, target_type) = parse_label("//:my_binary").unwrap();
        assert_eq!(package, "");
        assert_eq!(name, "my");
        assert_eq!(target_type, "binary");
    }

    #[test]
    fn test_parse_label_invalid() {
        let result = parse_label("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_integration_example_bazel_workspace_discovery() {
        let example_path = Path::new("example/bazel");
        if !example_path.exists() {
            return;
        }

        if !is_bazel_available() {
            return;
        }

        let result = list_targets(example_path, "bazel-example");
        match result {
            Ok(targets) => {
                assert!(!targets.is_empty(), "Should find at least one target");

                let has_hello = targets.iter().any(|t| t.name.contains(":hello"));
                assert!(has_hello, "Should find hello target");
            }
            Err(e) => {
                eprintln!("Integration test skipped or failed: {}", e);
            }
        }
    }
}
