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
//! - [`BazelTargetType`] - Distinguishes between binary and test targets
//! - [`is_bazel_available`] - Checks if `bazel` or `bazelisk` CLI is installed
//! - [`list_targets`] - Main function to list targets from a Bazel workspace
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! bazel query 'kind(".*_(binary|test)", //...)' --output=label_kind
//! ```
//!
//! And parses the `label_kind` output to extract targets with their rule types.
//! Each line has the format: `<rule_kind> rule <label>`, for example:
//! - `cc_binary rule //:hello`
//! - `cc_test rule //:hello_test`
//!
//! ## Execution
//!
//! Targets are executed based on their type:
//! - Binary targets: `bazel run <target>`
//! - Test targets: `bazel test <target>`
//!
//! The target type is propagated to the runner via a name prefix convention:
//! - `run:<label>` for binary targets
//! - `test:<label>` for test targets
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

/// Cache for resolved bazel/bazelisk binary name (checked once per process).
/// Contains `Some("bazelisk")` or `Some("bazel")` if available, `None` otherwise.
static BAZEL_BINARY: OnceLock<Option<&'static str>> = OnceLock::new();

/// The type of Bazel target (binary or test)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BazelTargetType {
    /// A binary target (`bazel run <label>`)
    Binary,
    /// A test target (`bazel test <label>`)
    Test,
}

/// Bazel target item for TUI display
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

/// Check if `bazel` or `bazelisk` is available.
pub fn is_bazel_available() -> bool {
    get_bazel_command().is_some()
}

/// Return the resolved binary name (`"bazelisk"` or `"bazel"`).
///
/// Prefers `bazelisk` over `bazel`. Returns `None` if neither is installed.
pub fn get_bazel_command() -> Option<&'static str> {
    *BAZEL_BINARY.get_or_init(|| {
        if Command::new("bazelisk")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some("bazelisk");
        }

        if Command::new("bazel")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some("bazel");
        }

        None
    })
}

/// Parse `bazel query --output=label_kind` output to extract binary and test targets.
///
/// Each line has the format: `<rule_kind> rule <label>`, for example:
/// - `cc_binary rule //:hello`
/// - `cc_test rule //:hello_test`
///
/// The rule kind determines the target type:
/// - Rules ending with `_binary` are binary targets
/// - Rules ending with `_test` are test targets
fn parse_bazel_query_output(output: &str, category: &str) -> Result<Vec<BazelTarget>> {
    let mut targets = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (rule_kind, label) = parse_label_kind_line(line)?;

        let target_type = if rule_kind.ends_with("_binary") {
            BazelTargetType::Binary
        } else if rule_kind.ends_with("_test") {
            BazelTargetType::Test
        } else {
            // Skip unknown rule kinds (shouldn't happen with our query filter)
            continue;
        };

        let (package, target_name) = parse_label(&label)?;

        let display_name = if package.is_empty() {
            format_display_name(&target_name)
        } else {
            format!("{}: {}", package, format_display_name(&target_name))
        };

        let description = match target_type {
            BazelTargetType::Binary => format!("bazel run {}", label),
            BazelTargetType::Test => format!("bazel test {}", label),
        };

        let emoji = match target_type {
            BazelTargetType::Binary => Some("\u{1f4e6}".to_string()), // 📦
            BazelTargetType::Test => Some("\u{1f4dd}".to_string()),   // 📝
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

/// Parse a single line of `bazel query --output=label_kind` output.
///
/// Expected format: `<rule_kind> rule <label>`
/// Returns `(rule_kind, label)`.
fn parse_label_kind_line(line: &str) -> Result<(String, String)> {
    // Format: "cc_binary rule //:hello"
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() != 3 || parts[1] != "rule" {
        anyhow::bail!(
            "Unexpected bazel query output format: '{}'. \
             Expected '<rule_kind> rule <label>'",
            line
        );
    }
    Ok((parts[0].to_string(), parts[2].to_string()))
}

/// Parse a Bazel label into its package and target name components.
///
/// Handles labels like:
/// - `//:hello` -> `("", "hello")`
/// - `//src/main:hello` -> `("src/main", "hello")`
/// - `//a/b/c` (shorthand for `//a/b/c:c`) -> `("a/b/c", "c")`
/// - `@repo//:hello` -> `("", "hello")`
fn parse_label(label: &str) -> Result<(String, String)> {
    // Strip optional external repository prefix (e.g., @repo)
    let label = if let Some(idx) = label.find("//") {
        &label[idx..]
    } else {
        label
    };

    if let Some(stripped) = label.strip_prefix("//") {
        if let Some((package, target_name)) = stripped.split_once(':') {
            Ok((package.to_string(), target_name.to_string()))
        } else {
            // Shorthand: //a/b/c means //a/b/c:c (last path component)
            let target_name = stripped.rsplit('/').next().unwrap_or(stripped).to_string();
            Ok((stripped.to_string(), target_name))
        }
    } else {
        anyhow::bail!("Invalid Bazel label: {}", label)
    }
}

/// Run `bazel query` and parse the result.
///
/// Discovers binary and test targets in the given Bazel workspace directory.
pub fn list_targets(workspace_dir: &Path, category: &str) -> Result<Vec<BazelTarget>> {
    let bazel_cmd = get_bazel_command().context("Bazel or Bazelisk not found")?;

    let output = Command::new(bazel_cmd)
        .arg("query")
        .arg("kind(\".*_(binary|test)\", //...)")
        .arg("--output=label_kind")
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
        r#"cc_binary rule //src/main:hello
cc_test rule //src/main:hello_test
cc_binary rule //examples/greeting:greeting
cc_test rule //examples/greeting:greeting_test"#
            .to_string()
    }

    #[test]
    fn test_parse_bazel_query_output() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        assert_eq!(targets.len(), 4);

        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"//src/main:hello"));
        assert!(names.contains(&"//src/main:hello_test"));
        assert!(names.contains(&"//examples/greeting:greeting"));
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

        // Binaries first (sorted alphabetically), then tests (sorted alphabetically)
        assert_eq!(targets[0].target_type, BazelTargetType::Binary);
        assert!(targets[0].name.contains("greeting"));
        assert_eq!(targets[1].target_type, BazelTargetType::Binary);
        assert!(targets[1].name.contains("hello"));
        assert_eq!(targets[2].target_type, BazelTargetType::Test);
        assert!(targets[2].name.contains("greeting"));
        assert_eq!(targets[3].target_type, BazelTargetType::Test);
        assert!(targets[3].name.contains("hello"));
    }

    #[test]
    fn test_parse_bazel_query_output_descriptions() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        let bin = targets
            .iter()
            .find(|t| t.name == "//src/main:hello")
            .unwrap();
        assert_eq!(bin.description, "bazel run //src/main:hello");

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
            .find(|t| t.name == "//src/main:hello")
            .unwrap();
        assert_eq!(hello.display_name, "src/main: Hello");

        let greeting = targets
            .iter()
            .find(|t| t.name == "//examples/greeting:greeting")
            .unwrap();
        assert_eq!(greeting.display_name, "examples/greeting: Greeting");
    }

    #[test]
    fn test_parse_bazel_query_output_emojis() {
        let output = sample_query_output();
        let targets = parse_bazel_query_output(&output, "myproject").unwrap();

        let bin = targets
            .iter()
            .find(|t| t.name == "//src/main:hello")
            .unwrap();
        assert_eq!(bin.emoji, Some("\u{1f4e6}".to_string())); // 📦

        let test = targets
            .iter()
            .find(|t| t.name == "//src/main:hello_test")
            .unwrap();
        assert_eq!(test.emoji, Some("\u{1f4dd}".to_string())); // 📝
    }

    #[test]
    fn test_parse_bazel_query_output_empty() {
        let targets = parse_bazel_query_output("", "myproject").unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_label_kind_line_binary() {
        let (kind, label) = parse_label_kind_line("cc_binary rule //:hello").unwrap();
        assert_eq!(kind, "cc_binary");
        assert_eq!(label, "//:hello");
    }

    #[test]
    fn test_parse_label_kind_line_test() {
        let (kind, label) = parse_label_kind_line("cc_test rule //src:hello_test").unwrap();
        assert_eq!(kind, "cc_test");
        assert_eq!(label, "//src:hello_test");
    }

    #[test]
    fn test_parse_label_kind_line_java() {
        let (kind, label) = parse_label_kind_line("java_binary rule //src/main/java:app").unwrap();
        assert_eq!(kind, "java_binary");
        assert_eq!(label, "//src/main/java:app");
    }

    #[test]
    fn test_parse_label_kind_line_invalid() {
        let result = parse_label_kind_line("invalid format");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_label_with_colon() {
        let (package, name) = parse_label("//src/main:hello").unwrap();
        assert_eq!(package, "src/main");
        assert_eq!(name, "hello");
    }

    #[test]
    fn test_parse_label_root() {
        let (package, name) = parse_label("//:hello").unwrap();
        assert_eq!(package, "");
        assert_eq!(name, "hello");
    }

    #[test]
    fn test_parse_label_shorthand() {
        // //a/b/c is shorthand for //a/b/c:c
        let (package, name) = parse_label("//a/b/c").unwrap();
        assert_eq!(package, "a/b/c");
        assert_eq!(name, "c");
    }

    #[test]
    fn test_parse_label_with_at_prefix() {
        let (package, name) = parse_label("@repo//:hello").unwrap();
        assert_eq!(package, "");
        assert_eq!(name, "hello");
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

        let targets =
            list_targets(example_path, "bazel-example").expect("bazel query should succeed");
        assert!(!targets.is_empty(), "Should find at least one target");

        let has_hello = targets.iter().any(|t| t.label.contains(":hello"));
        assert!(has_hello, "Should find hello target");

        // Verify the hello binary target exists and has the correct type
        let hello = targets.iter().find(|t| t.label == "//:hello").unwrap();
        assert_eq!(hello.target_type, BazelTargetType::Binary);

        // Verify the hello_test target exists and has the correct type
        let hello_test = targets.iter().find(|t| t.label == "//:hello_test").unwrap();
        assert_eq!(hello_test.target_type, BazelTargetType::Test);
    }
}
