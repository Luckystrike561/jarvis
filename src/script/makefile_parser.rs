//! # Makefile Parser
//!
//! This module parses Makefile configurations and extracts available targets.
//!
//! ## Overview
//!
//! Unlike other parsers that read configuration files directly, this parser
//! invokes `make --print-data-base` to get target information. This approach:
//!
//! - Handles complex Makefile includes and variables
//! - Respects standard Makefile patterns
//! - Gets accurate target names including those with dependencies
//!
//! Additionally, this parser reads comments from the Makefile to extract
//! annotations for customizing target display in the TUI.
//!
//! ## Key Types
//!
//! - [`MakeTarget`] - Represents a make target with display metadata for the TUI
//! - [`MakeAnnotations`] - Annotations extracted from Makefile comments
//! - [`is_make_available`] - Checks if `make` CLI is installed
//! - [`list_targets`] - Main function to list targets from a Makefile
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! make --print-data-base --dry-run --file <path> 2>/dev/null
//! ```
//!
//! And parses the output which includes target information.
//!
//! ## Annotations
//!
//! Targets can be annotated with special comments above their definitions:
//!
//! ```makefile
//! # @emoji 🚀
//! # @description Deploy the application to production
//! deploy:
//!     ./deploy.sh
//!
//! # @ignore
//! _internal_helper:
//!     @echo "Internal helper - hidden from TUI"
//! ```
//!
//! ### Available Annotations
//!
//! | Annotation | Description |
//! |------------|-------------|
//! | `@emoji <emoji>` | Display emoji prefix in the TUI |
//! | `@description <text>` | Custom description for the details panel |
//! | `@ignore` | Hide the target from the TUI |
//!
//! ## Availability Caching
//!
//! The `make` binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use regex::Regex;

use crate::script::discovery::format_display_name;

/// Cache for make availability check (checked once per process)
static MAKE_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Special Makefile target names that should be excluded from the TUI
const SKIP_PATTERNS: &[&str] = &[
    ".PHONY",
    ".DEFAULT",
    ".PRECIOUS",
    ".INTERMEDIATE",
    ".SECONDARY",
    ".SECONDEXPANSION",
    ".DELETE_ON_ERROR",
    ".IGNORE",
    ".LOW_RESOLUTION_TIME",
    ".SILENT",
    ".EXPORT_ALL_VARIABLES",
    ".NOTPARALLEL",
    ".ONESHELL",
    ".POSIX",
];

/// File extensions that indicate build artifact targets to be filtered out
const ARTIFACT_EXTENSIONS: &[&str] = &[
    ".o", ".a", ".so", ".out", ".obj", ".lib", ".dll", ".dylib", ".exe",
];

/// Make target item for TUI display (mirrors other script types)
#[derive(Debug, Clone)]
pub struct MakeTarget {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
}

/// Annotations extracted from Makefile comments above a target definition
#[derive(Debug, Clone, Default)]
pub struct MakeAnnotations {
    pub emoji: Option<String>,
    pub description: Option<String>,
    pub ignored: bool,
}

/// Check if the `make` binary is available.
pub fn is_make_available() -> bool {
    *MAKE_AVAILABLE.get_or_init(|| {
        Command::new("make")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Parse annotations from Makefile comments.
///
/// Looks for special comment annotations above target definitions:
/// - `# @emoji <emoji>` - Display emoji prefix in the TUI
/// - `# @description <text>` - Custom description for the details panel
/// - `# @ignore` - Hide the target from the TUI
///
/// Returns a map of target names to their annotations.
pub fn parse_makefile_annotations(
    makefile_path: &Path,
) -> Result<HashMap<String, MakeAnnotations>> {
    let content = fs::read_to_string(makefile_path)
        .with_context(|| format!("Failed to read Makefile: {}", makefile_path.display()))?;

    parse_makefile_annotations_from_content(&content)
}

/// Parse annotations from Makefile content (for testing).
pub fn parse_makefile_annotations_from_content(
    content: &str,
) -> Result<HashMap<String, MakeAnnotations>> {
    let mut annotations_map: HashMap<String, MakeAnnotations> = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();

    // Regex patterns for annotations (same as other parsers)
    let emoji_re =
        Regex::new(r"^\s*#\s*@emoji\s+(.+)$").context("Failed to compile emoji regex pattern")?;
    let desc_re = Regex::new(r"^\s*#\s*@description\s+(.+)$")
        .context("Failed to compile description regex pattern")?;
    let ignore_re =
        Regex::new(r"^\s*#\s*@ignore\s*$").context("Failed to compile ignore regex pattern")?;
    let comment_re = Regex::new(r"^\s*#").context("Failed to compile comment regex pattern")?;
    let plain_comment_re =
        Regex::new(r"^\s*#\s+(.+)$").context("Failed to compile plain comment regex pattern")?;

    // Regex to match target definitions in Makefile
    // Targets are lines that look like: "target_name:" or "target_name: dependencies"
    // Must not start with whitespace (those are recipe lines)
    // Must not be a variable assignment (contains = without : before it)
    let target_def_re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)\s*:")
        .context("Failed to compile target definition regex")?;

    for (line_idx, line) in lines.iter().enumerate() {
        // Match target definitions (lines that start with a target name followed by colon)
        if let Some(cap) = target_def_re.captures(line) {
            let target_name = &cap[1];

            // Skip special targets (like .PHONY, .DEFAULT, etc.)
            if target_name.starts_with('.') {
                continue;
            }

            // Extract annotations from preceding comment lines
            let mut emoji: Option<String> = None;
            let mut description: Option<String> = None;
            let mut plain_comment: Option<String> = None;
            let mut ignored = false;

            // Look backwards from the target line through consecutive comment lines
            let mut check_idx = line_idx.saturating_sub(1);
            loop {
                if check_idx >= line_idx {
                    break; // Underflow protection
                }

                let prev_line = lines[check_idx];

                // If we hit an empty line or a non-comment line, stop looking back
                if prev_line.trim().is_empty() || !comment_re.is_match(prev_line) {
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

                // Check for plain comment (not an annotation) as fallback description
                if plain_comment.is_none()
                    && !ignore_re.is_match(prev_line)
                    && !emoji_re.is_match(prev_line)
                    && !desc_re.is_match(prev_line)
                {
                    if let Some(plain_cap) = plain_comment_re.captures(prev_line) {
                        let text = plain_cap[1].trim().to_string();
                        if !text.is_empty() {
                            plain_comment = Some(text);
                        }
                    }
                }

                if check_idx == 0 {
                    break;
                }
                check_idx -= 1;
            }

            // Use @description if present, otherwise fall back to plain comment
            let final_description = description.or(plain_comment);

            // Add if there are any annotations or a plain comment description
            if emoji.is_some() || final_description.is_some() || ignored {
                annotations_map.insert(
                    target_name.to_string(),
                    MakeAnnotations {
                        emoji,
                        description: final_description,
                        ignored,
                    },
                );
            }
        }
    }

    Ok(annotations_map)
}

/// Check if a target name looks like a build artifact based on file extensions.
fn is_artifact_target(name: &str) -> bool {
    ARTIFACT_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

/// Parse output from `make --print-data-base` to extract target names.
///
/// This function parses the make database output to find all targets,
/// filtering out built-in implicit rules and build artifact targets.
fn parse_make_database(
    output: &str,
    category: &str,
    annotations: Option<&HashMap<String, MakeAnnotations>>,
) -> Result<Vec<MakeTarget>> {
    let mut targets = Vec::new();
    let mut seen_targets = HashSet::new();
    let mut not_a_target_names = HashSet::new();

    // Regex to match target definitions in make database output
    // The database shows targets in a specific format
    let target_re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)\s*:")
        .context("Failed to compile target regex for make database")?;

    // First pass: collect targets marked as "# Not a target:" in the database
    // The make database output uses this marker for built-in implicit rules
    let lines: Vec<&str> = output.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("# Not a target:") {
            // The target name is on the next line
            if let Some(next_line) = lines.get(i + 1) {
                if let Some(cap) = target_re.captures(next_line) {
                    not_a_target_names.insert(cap[1].to_string());
                }
            }
        }
    }

    for line in output.lines() {
        // Skip empty lines and comments
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        // Try to match a target definition
        if let Some(cap) = target_re.captures(line) {
            let target_name = cap[1].to_string();

            // Skip special targets
            if SKIP_PATTERNS.contains(&target_name.as_str()) || target_name.starts_with('.') {
                continue;
            }

            // Skip built-in targets identified by "# Not a target:" markers
            if not_a_target_names.contains(&target_name) {
                continue;
            }

            // Skip targets that look like build artifacts (contain file extensions)
            if is_artifact_target(&target_name) {
                continue;
            }

            // Skip if we've already seen this target
            if !seen_targets.insert(target_name.clone()) {
                continue;
            }

            // Look up annotations for this target
            let target_annotations = annotations.and_then(|a| a.get(&target_name));

            let display_name = format_display_name(&target_name);

            // Use annotation description if available, otherwise provide default
            let description = target_annotations
                .and_then(|a| a.description.clone())
                .unwrap_or_else(|| format!("make target {}", target_name));

            let emoji = target_annotations.and_then(|a| a.emoji.clone());
            let ignored = target_annotations.is_some_and(|a| a.ignored);

            targets.push(MakeTarget {
                name: target_name,
                display_name,
                category: category.to_string(),
                description,
                emoji,
                ignored,
            });
        }
    }

    targets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(targets)
}

/// Run `make --print-data-base` and parse the result.
///
/// Also parses annotations from the Makefile comments.
pub fn list_targets(makefile_path: &Path, category: &str) -> Result<Vec<MakeTarget>> {
    // First, parse annotations from the Makefile
    let annotations = parse_makefile_annotations(makefile_path).ok();

    let output = Command::new("make")
        .arg("--print-data-base")
        .arg("--dry-run")
        .arg("--file")
        .arg(makefile_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("Failed to run make for: {}", makefile_path.display()))?;

    if !output.status.success() {
        // If make --print-data-base fails, try a simpler approach:
        // just parse the Makefile directly for target names
        return list_targets_from_parsing(makefile_path, category, annotations.as_ref());
    }

    let output_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).to_string(),
    };

    // Parse the database output
    parse_make_database(&output_str, category, annotations.as_ref())
}

/// Fallback: Parse Makefile directly to extract target names.
///
/// This is used when `make --print-data-base` fails or returns no output.
fn list_targets_from_parsing(
    makefile_path: &Path,
    category: &str,
    annotations: Option<&HashMap<String, MakeAnnotations>>,
) -> Result<Vec<MakeTarget>> {
    let content = fs::read_to_string(makefile_path)
        .with_context(|| format!("Failed to read Makefile: {}", makefile_path.display()))?;

    let mut targets = Vec::new();
    let mut seen_targets = HashSet::new();

    // Regex to match target definitions
    // Format: "target_name:" or "target_name: dependencies"
    // Must not start with whitespace (those are recipe lines)
    let target_re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)\s*:")?;

    for line in content.lines() {
        // Skip empty lines, comments, and lines starting with whitespace (recipe lines)
        if line.trim().is_empty()
            || line.trim_start().starts_with('#')
            || line.starts_with('\t')
            || line.starts_with(' ')
        {
            continue;
        }

        // Try to match a target definition
        if let Some(cap) = target_re.captures(line) {
            let target_name = cap[1].to_string();

            // Skip special targets
            if SKIP_PATTERNS.contains(&target_name.as_str()) || target_name.starts_with('.') {
                continue;
            }

            // Skip targets that look like build artifacts
            if is_artifact_target(&target_name) {
                continue;
            }

            // Skip variable assignments (lines with = before the colon)
            if line.contains('=') {
                if let (Some(eq_pos), Some(colon_pos)) = (line.find('='), line.find(':')) {
                    if eq_pos < colon_pos {
                        continue; // This is a variable assignment, not a target
                    }
                }
            }

            // Skip if we've already seen this target
            if !seen_targets.insert(target_name.clone()) {
                continue;
            }

            // Look up annotations for this target
            let target_annotations = annotations.and_then(|a| a.get(&target_name));

            let display_name = format_display_name(&target_name);

            // Use annotation description if available, otherwise provide default
            let description = target_annotations
                .and_then(|a| a.description.clone())
                .unwrap_or_else(|| format!("make target {}", target_name));

            let emoji = target_annotations.and_then(|a| a.emoji.clone());
            let ignored = target_annotations.is_some_and(|a| a.ignored);

            targets.push(MakeTarget {
                name: target_name,
                display_name,
                category: category.to_string(),
                description,
                emoji,
                ignored,
            });
        }
    }

    targets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_makefile_annotations_emoji() {
        let content = r#"# Makefile for my project

# @emoji 🚀
deploy:
	./deploy.sh
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy"));
        assert_eq!(annotations["deploy"].emoji, Some("🚀".to_string()));
        assert!(!annotations["deploy"].ignored);
    }

    #[test]
    fn test_parse_makefile_annotations_description() {
        let content = r#"# Makefile

# @description Build the project with optimizations
build:
	cargo build --release
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("build"));
        assert_eq!(
            annotations["build"].description,
            Some("Build the project with optimizations".to_string())
        );
    }

    #[test]
    fn test_parse_makefile_annotations_ignore() {
        let content = r#"# Makefile

# @ignore
_helper:
	@echo "Internal helper"
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("_helper"));
        assert!(annotations["_helper"].ignored);
    }

    #[test]
    fn test_parse_makefile_annotations_multiple() {
        let content = r#"# Makefile

# @emoji 🔧
# @description Build the project
build:
	cargo build

# @ignore
_clean:
	@rm -rf target

# @emoji 🧪
test:
	cargo test
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();

        assert!(annotations.contains_key("build"));
        assert_eq!(annotations["build"].emoji, Some("🔧".to_string()));
        assert_eq!(
            annotations["build"].description,
            Some("Build the project".to_string())
        );
        assert!(!annotations["build"].ignored);

        assert!(annotations.contains_key("_clean"));
        assert!(annotations["_clean"].ignored);

        assert!(annotations.contains_key("test"));
        assert_eq!(annotations["test"].emoji, Some("🧪".to_string()));
        assert!(!annotations["test"].ignored);
    }

    #[test]
    fn test_parse_makefile_annotations_no_annotations() {
        let content = r#"# Makefile

build:
	cargo build

test:
	cargo test
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(!annotations.contains_key("build"));
        assert!(!annotations.contains_key("test"));
    }

    #[test]
    fn test_parse_makefile_annotations_skips_special_targets() {
        let content = r#"# Makefile

.PHONY: all clean

# @emoji 🚀
all:
	@echo "Building all"
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        // .PHONY should not be in annotations (it's a special target)
        assert!(!annotations.contains_key(".PHONY"));
        // 'all' should be in annotations
        assert!(annotations.contains_key("all"));
        assert_eq!(annotations["all"].emoji, Some("🚀".to_string()));
    }

    #[test]
    fn test_parse_makefile_annotations_hyphenated_targets() {
        let content = r#"# Makefile

# @emoji 🚀
deploy-production:
	./deploy.sh

# @emoji 🧪
run-integration-tests:
	cargo test --features integration
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy-production"));
        assert_eq!(
            annotations["deploy-production"].emoji,
            Some("🚀".to_string())
        );
        assert!(annotations.contains_key("run-integration-tests"));
        assert_eq!(
            annotations["run-integration-tests"].emoji,
            Some("🧪".to_string())
        );
    }

    #[test]
    fn test_parse_makefile_annotations_ignores_recipe_lines() {
        let content = r#"# Makefile

build:
	# This is a recipe comment, not an annotation
	cargo build
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        // Recipe comments should not be picked up as annotations for 'build'
        // because they're on recipe lines (start with tab)
        assert!(!annotations.contains_key("cargo"));
        // But 'build' itself should not have any annotations
        // because there's no comment directly above it
        assert!(!annotations.contains_key("build"));
    }

    #[test]
    fn test_list_targets_from_parsing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let makefile_path = temp_dir.path().join("Makefile");

        let content = r#"# Makefile

.PHONY: all clean test

all: build test

build:
	cargo build

test:
	cargo test

clean:
	rm -rf target

# @emoji 🚀
deploy:
	./deploy.sh
"#;
        std::fs::write(&makefile_path, content).unwrap();

        // Parse annotations from the Makefile
        let annotations = parse_makefile_annotations(&makefile_path).ok();

        let targets =
            list_targets_from_parsing(&makefile_path, "myproject", annotations.as_ref()).unwrap();

        // Should find: all, build, test, clean, deploy
        // Should NOT find: .PHONY
        assert!(
            targets.iter().any(|t| t.name == "all"),
            "Should find 'all' target"
        );
        assert!(
            targets.iter().any(|t| t.name == "build"),
            "Should find 'build' target"
        );
        assert!(
            targets.iter().any(|t| t.name == "test"),
            "Should find 'test' target"
        );
        assert!(
            targets.iter().any(|t| t.name == "clean"),
            "Should find 'clean' target"
        );
        assert!(
            targets.iter().any(|t| t.name == "deploy"),
            "Should find 'deploy' target"
        );
        assert!(
            !targets.iter().any(|t| t.name == ".PHONY"),
            "Should NOT find '.PHONY' target"
        );

        // Check that deploy has the emoji annotation
        let deploy = targets.iter().find(|t| t.name == "deploy").unwrap();
        assert_eq!(deploy.emoji, Some("🚀".to_string()));
    }

    #[test]
    fn test_list_targets_from_parsing_skips_variables() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let makefile_path = temp_dir.path().join("Makefile");

        let content = r#"# Makefile

CC = gcc
CFLAGS = -Wall

build:
	$(CC) $(CFLAGS) -o output main.c
"#;
        std::fs::write(&makefile_path, content).unwrap();

        let targets = list_targets_from_parsing(&makefile_path, "myproject", None).unwrap();

        // Should only find 'build', not 'CC' or 'CFLAGS'
        assert!(targets.iter().any(|t| t.name == "build"));
        assert!(!targets.iter().any(|t| t.name == "CC"));
        assert!(!targets.iter().any(|t| t.name == "CFLAGS"));
    }

    #[test]
    fn test_parse_makefile_annotations_plain_comment_description() {
        let content = r#"# Makefile

# Build the project with cargo
build:
	cargo build

# Run all tests
test:
	cargo test
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("build"));
        assert_eq!(
            annotations["build"].description,
            Some("Build the project with cargo".to_string())
        );
        assert!(annotations.contains_key("test"));
        assert_eq!(
            annotations["test"].description,
            Some("Run all tests".to_string())
        );
    }

    #[test]
    fn test_parse_makefile_annotations_description_overrides_plain_comment() {
        let content = r#"# Makefile

# This plain comment should be overridden
# @description Official description
build:
	cargo build
"#;

        let annotations = parse_makefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("build"));
        assert_eq!(
            annotations["build"].description,
            Some("Official description".to_string())
        );
    }

    #[test]
    fn test_parse_make_database_filters_not_a_target() {
        let db_output = "# Not a target:\nar:\n#  Implicit rule search has not been done.\n\nbuild:\n#  Phony target (prerequisite of .PHONY).\n";

        let targets = parse_make_database(db_output, "test", None).unwrap();

        // 'ar' should be filtered out because it's marked as "# Not a target:"
        assert!(
            !targets.iter().any(|t| t.name == "ar"),
            "Should NOT find 'ar' (built-in target)"
        );
        // 'build' should be present
        assert!(
            targets.iter().any(|t| t.name == "build"),
            "Should find 'build' target"
        );
    }

    #[test]
    fn test_parse_make_database_filters_artifact_targets() {
        let db_output = "build:\nmain.o:\nlibfoo.a:\napp.so:\nresult.out:\n";

        let targets = parse_make_database(db_output, "test", None).unwrap();

        assert!(
            targets.iter().any(|t| t.name == "build"),
            "Should find 'build' target"
        );
        assert!(
            !targets.iter().any(|t| t.name == "main"),
            "Should NOT find artifact target 'main.o'"
        );
        assert!(
            !targets.iter().any(|t| t.name.contains('.')),
            "Should NOT find any artifact targets with extensions"
        );
    }

    #[test]
    fn test_is_artifact_target() {
        assert!(is_artifact_target("main.o"));
        assert!(is_artifact_target("libfoo.a"));
        assert!(is_artifact_target("app.so"));
        assert!(is_artifact_target("program.out"));
        assert!(is_artifact_target("mylib.dll"));
        assert!(is_artifact_target("mylib.dylib"));
        assert!(is_artifact_target("app.exe"));
        assert!(!is_artifact_target("build"));
        assert!(!is_artifact_target("deploy-production"));
        assert!(!is_artifact_target("run_tests"));
    }

    #[test]
    fn test_list_targets_from_parsing_skips_artifact_targets() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let makefile_path = temp_dir.path().join("Makefile");

        let content = r#"# Makefile

build:
	cargo build

main.o:
	gcc -c main.c

libfoo.a:
	ar rcs libfoo.a foo.o
"#;
        std::fs::write(&makefile_path, content).unwrap();

        let targets = list_targets_from_parsing(&makefile_path, "myproject", None).unwrap();

        assert!(
            targets.iter().any(|t| t.name == "build"),
            "Should find 'build' target"
        );
        // Targets with artifact extensions should be filtered
        // Note: the regex requires targets to start with [a-zA-Z_] so "main.o" won't match anyway,
        // but if we had "main_obj.o" it would be caught by is_artifact_target
        assert!(
            !targets.iter().any(|t| is_artifact_target(&t.name)),
            "Should NOT find any artifact targets"
        );
    }
}
