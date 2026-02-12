//! # Just (justfile) Parser
//!
//! This module parses justfile configurations and extracts available recipes.
//!
//! ## Overview
//!
//! Unlike other parsers that read configuration files directly, this parser
//! invokes `just --list` to get recipe information. This approach:
//!
//! - Handles complex justfile includes and imports
//! - Respects recipe visibility and private recipes
//! - Gets accurate recipe names and descriptions from the CLI
//!
//! Additionally, this parser reads comments from the justfile to extract
//! annotations for customizing recipe display in the TUI.
//!
//! ## Key Types
//!
//! - [`JustRecipe`] - Represents a just recipe with display metadata for the TUI
//! - [`JustAnnotations`] - Annotations extracted from justfile comments
//! - [`is_just_available`] - Checks if `just` CLI is installed
//! - [`list_recipes`] - Main function to list recipes from a justfile
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! just --list --unsorted --list-heading '' --list-prefix '' --justfile <path>
//! ```
//!
//! And parses the output which includes recipe names and optional descriptions.
//!
//! ## Annotations
//!
//! Recipes can be annotated with special comments above their definitions:
//!
//! ```just
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
//! | `@ignore` | Hide the recipe from the TUI |
//!
//! ## Availability Caching
//!
//! The `just` binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::script::discovery::format_display_name;

/// Cache for just availability check (checked once per process)
static JUST_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Just recipe item for TUI display (mirrors other script types)
#[derive(Debug, Clone)]
pub struct JustRecipe {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
}

/// Annotations extracted from justfile comments above a recipe definition
#[derive(Debug, Clone, Default)]
pub struct JustAnnotations {
    pub emoji: Option<String>,
    pub description: Option<String>,
    pub ignored: bool,
}

/// Check if the `just` binary is available.
pub fn is_just_available() -> bool {
    *JUST_AVAILABLE.get_or_init(|| {
        Command::new("just")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Parse annotations from justfile comments.
///
/// Looks for special comment annotations above recipe definitions:
/// - `# @emoji <emoji>` - Display emoji prefix in the TUI
/// - `# @description <text>` - Custom description for the details panel
/// - `# @ignore` - Hide the recipe from the TUI
///
/// Returns a map of recipe names to their annotations.
pub fn parse_justfile_annotations(
    justfile_path: &Path,
) -> Result<HashMap<String, JustAnnotations>> {
    let content = fs::read_to_string(justfile_path)
        .with_context(|| format!("Failed to read justfile: {}", justfile_path.display()))?;

    parse_justfile_annotations_from_content(&content)
}

/// Parse annotations from justfile content (for testing).
pub fn parse_justfile_annotations_from_content(
    content: &str,
) -> Result<HashMap<String, JustAnnotations>> {
    let mut annotations_map: HashMap<String, JustAnnotations> = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();

    // Regex patterns for annotations (same as other parsers)
    let emoji_re =
        Regex::new(r"^\s*#\s*@emoji\s+(.+)$").context("Failed to compile emoji regex pattern")?;
    let desc_re = Regex::new(r"^\s*#\s*@description\s+(.+)$")
        .context("Failed to compile description regex pattern")?;
    let ignore_re =
        Regex::new(r"^\s*#\s*@ignore\s*$").context("Failed to compile ignore regex pattern")?;
    let comment_re = Regex::new(r"^\s*#").context("Failed to compile comment regex pattern")?;

    // Regex to match recipe definitions in justfile
    // Recipes are lines that start with a name followed by optional parameters and a colon
    // Must not start with whitespace (those are recipe body lines)
    // Examples: "build:", "deploy env=\"staging\":", "test *args:"
    let recipe_line_re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)(?:\s+[^:]*)?:\s*")
        .context("Failed to compile recipe line regex")?;

    for (line_idx, line) in lines.iter().enumerate() {
        // Skip lines starting with whitespace (recipe body), empty lines, comments
        if line.starts_with(' ')
            || line.starts_with('\t')
            || line.trim().is_empty()
            || comment_re.is_match(line)
        {
            continue;
        }

        // Match recipe definitions
        if let Some(cap) = recipe_line_re.captures(line) {
            let recipe_name = &cap[1];

            // Skip variable assignments (lines with := or = before the colon)
            if line.contains(":=") || (line.contains('=') && !line.contains(':')) {
                continue;
            }

            // Skip 'set', 'alias', 'export', 'import', 'mod' keywords
            if matches!(
                recipe_name,
                "set" | "alias" | "export" | "import" | "mod" | "if" | "else"
            ) {
                continue;
            }

            // Extract annotations from preceding comment lines
            let mut emoji: Option<String> = None;
            let mut description: Option<String> = None;
            let mut ignored = false;

            // Look backwards from the recipe line through consecutive comment lines
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
                    recipe_name.to_string(),
                    JustAnnotations {
                        emoji,
                        description,
                        ignored,
                    },
                );
            }
        }
    }

    Ok(annotations_map)
}

/// Parse output from `just --list` to extract recipe names and descriptions.
///
/// The output format is:
/// ```text
/// recipe-name # optional description
/// another-recipe
/// recipe-with-args arg1 arg2 # builds the project
/// ```
pub fn parse_just_list_output(
    output: &str,
    category: &str,
    annotations: Option<&HashMap<String, JustAnnotations>>,
) -> Result<Vec<JustRecipe>> {
    let mut recipes = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse the line: "recipe-name [args...] # description"
        // Split on '#' first to get description
        let (name_part, comment) = if let Some(hash_pos) = line.find('#') {
            (&line[..hash_pos], Some(line[hash_pos + 1..].trim()))
        } else {
            (line, None)
        };

        // The recipe name is the first word
        let name_part = name_part.trim();
        let recipe_name = match name_part.split_whitespace().next() {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Look up annotations for this recipe
        let recipe_annotations = annotations.and_then(|a| a.get(&recipe_name));

        let display_name = format_display_name(&recipe_name);

        // Use annotation description, then just comment, then default
        let description = recipe_annotations
            .and_then(|a| a.description.clone())
            .or_else(|| comment.map(|c| c.to_string()))
            .unwrap_or_else(|| format!("just recipe {}", recipe_name));

        let emoji = recipe_annotations.and_then(|a| a.emoji.clone());
        let ignored = recipe_annotations.is_some_and(|a| a.ignored);

        recipes.push(JustRecipe {
            name: recipe_name,
            display_name,
            category: category.to_string(),
            description,
            emoji,
            ignored,
        });
    }

    recipes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(recipes)
}

/// Run `just --list` and parse the result.
///
/// Also parses annotations from the justfile comments.
pub fn list_recipes(justfile_path: &Path, category: &str) -> Result<Vec<JustRecipe>> {
    // First, parse annotations from the justfile
    let annotations = parse_justfile_annotations(justfile_path).ok();

    let output = Command::new("just")
        .arg("--list")
        .arg("--unsorted")
        .arg("--list-heading")
        .arg("")
        .arg("--list-prefix")
        .arg("")
        .arg("--justfile")
        .arg(justfile_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("Failed to run just for: {}", justfile_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "just --list failed for {}: {}",
            justfile_path.display(),
            stderr,
        );
    }

    let output_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).to_string(),
    };

    parse_just_list_output(&output_str, category, annotations.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_just_list_output_simple() {
        let output = "build\ntest\nclean\n";
        let result = parse_just_list_output(output, "myproject", None).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "build");
        assert_eq!(result[0].display_name, "Build");
        assert_eq!(result[0].category, "myproject");
        assert_eq!(result[0].description, "just recipe build");
        assert_eq!(result[0].emoji, None);
        assert!(!result[0].ignored);
    }

    #[test]
    fn test_parse_just_list_output_with_descriptions() {
        let output = "build # Build the project\ntest # Run tests\n";
        let result = parse_just_list_output(output, "myproject", None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "build");
        assert_eq!(result[0].description, "Build the project");
        assert_eq!(result[1].name, "test");
        assert_eq!(result[1].description, "Run tests");
    }

    #[test]
    fn test_parse_just_list_output_with_args() {
        let output = "deploy env=\"staging\" # Deploy to environment\nbuild\n";
        let result = parse_just_list_output(output, "cat", None).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "build");
        assert_eq!(result[1].name, "deploy");
        assert_eq!(result[1].description, "Deploy to environment");
    }

    #[test]
    fn test_parse_just_list_output_sorted() {
        let output = "z_last\na_first\nm_middle\n";
        let result = parse_just_list_output(output, "cat", None).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "a_first");
        assert_eq!(result[1].name, "m_middle");
        assert_eq!(result[2].name, "z_last");
    }

    #[test]
    fn test_parse_just_list_output_empty() {
        let output = "";
        let result = parse_just_list_output(output, "cat", None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_just_list_output_with_annotations() {
        let output = "build\ndeploy\n";

        let mut annotations = HashMap::new();
        annotations.insert(
            "deploy".to_string(),
            JustAnnotations {
                emoji: Some("🚀".to_string()),
                description: Some("Custom deploy description".to_string()),
                ignored: false,
            },
        );

        let result = parse_just_list_output(output, "cat", Some(&annotations)).unwrap();
        assert_eq!(result.len(), 2);

        // build should have no annotations
        assert_eq!(result[0].name, "build");
        assert_eq!(result[0].emoji, None);
        assert_eq!(result[0].description, "just recipe build");
        assert!(!result[0].ignored);

        // deploy should have annotations applied
        assert_eq!(result[1].name, "deploy");
        assert_eq!(result[1].emoji, Some("🚀".to_string()));
        assert_eq!(result[1].description, "Custom deploy description");
        assert!(!result[1].ignored);
    }

    #[test]
    fn test_parse_just_list_output_annotation_overrides_comment() {
        let output = "build # Original description from just\n";

        let mut annotations = HashMap::new();
        annotations.insert(
            "build".to_string(),
            JustAnnotations {
                emoji: None,
                description: Some("Overridden description from annotation".to_string()),
                ignored: false,
            },
        );

        let result = parse_just_list_output(output, "cat", Some(&annotations)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].description,
            "Overridden description from annotation"
        );
    }

    #[test]
    fn test_parse_justfile_annotations_emoji() {
        let content = r#"# Build the project

# @emoji 🚀
deploy:
    ./deploy.sh
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy"));
        assert_eq!(annotations["deploy"].emoji, Some("🚀".to_string()));
        assert!(!annotations["deploy"].ignored);
    }

    #[test]
    fn test_parse_justfile_annotations_description() {
        let content = r#"# @description Build the project with optimizations
build:
    cargo build --release
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("build"));
        assert_eq!(
            annotations["build"].description,
            Some("Build the project with optimizations".to_string())
        );
    }

    #[test]
    fn test_parse_justfile_annotations_ignore() {
        let content = r#"# @ignore
_helper:
    @echo "Internal helper"
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("_helper"));
        assert!(annotations["_helper"].ignored);
    }

    #[test]
    fn test_parse_justfile_annotations_multiple() {
        let content = r#"# @emoji 🔧
# @description Build the project
build:
    cargo build

# @ignore
_clean:
    rm -rf target

# @emoji 🧪
test:
    cargo test
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();

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
    fn test_parse_justfile_annotations_no_annotations() {
        let content = r#"build:
    cargo build

test:
    cargo test
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(!annotations.contains_key("build"));
        assert!(!annotations.contains_key("test"));
    }

    #[test]
    fn test_parse_justfile_annotations_all_three() {
        let content = r#"# @emoji 🚀
# @description Deploy to production
# @ignore
deploy_internal:
    ./deploy.sh
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy_internal"));
        assert_eq!(annotations["deploy_internal"].emoji, Some("🚀".to_string()));
        assert_eq!(
            annotations["deploy_internal"].description,
            Some("Deploy to production".to_string())
        );
        assert!(annotations["deploy_internal"].ignored);
    }

    #[test]
    fn test_parse_justfile_annotations_with_parameters() {
        let content = r#"# @emoji 🚀
deploy env="staging":
    ./deploy.sh {{env}}

# @emoji 🧪
test *args:
    cargo test {{args}}
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy"));
        assert_eq!(annotations["deploy"].emoji, Some("🚀".to_string()));
        assert!(annotations.contains_key("test"));
        assert_eq!(annotations["test"].emoji, Some("🧪".to_string()));
    }

    #[test]
    fn test_parse_justfile_annotations_hyphenated_names() {
        let content = r#"# @emoji 🚀
deploy-production:
    ./deploy.sh

# @emoji 🧪
run-integration-tests:
    cargo test --features integration
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
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
    fn test_parse_justfile_annotations_skips_variables() {
        let content = r#"name := "world"

# @emoji 🚀
build:
    cargo build
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(!annotations.contains_key("name"));
        assert!(annotations.contains_key("build"));
    }

    #[test]
    fn test_parse_justfile_annotations_skips_keywords() {
        let content = r#"set shell := ["bash", "-c"]

# @emoji 🚀
build:
    cargo build
"#;

        let annotations = parse_justfile_annotations_from_content(content).unwrap();
        assert!(!annotations.contains_key("set"));
        assert!(annotations.contains_key("build"));
    }

    #[test]
    fn test_parse_just_list_output_whitespace_lines() {
        let output = "  build  \n  test  \n\n";
        let result = parse_just_list_output(output, "cat", None).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "build");
        assert_eq!(result[1].name, "test");
    }
}
