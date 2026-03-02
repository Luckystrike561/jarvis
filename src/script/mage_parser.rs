//! # Mage Parser
//!
//! This module parses magefile configurations and extracts available targets.
//!
//! ## Overview
//!
//! This parser invokes `mage -l` to get target information. This approach handles
//! complex magefile structures and gets accurate target names from the CLI.
//!
//! ## Key Types
//!
//! - [`MageTarget`] - Represents a mage target with display metadata for the TUI
//! - [`MageAnnotations`] - Annotations extracted from magefile comments
//! - [`is_mage_available`] - Checks if `mage` CLI is installed
//! - [`list_targets`] - Main function to list targets from a magefile
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! mage -l
//! ```

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::script::discovery::format_display_name;

/// Cache for mage availability check (checked once per process)
static MAGE_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Module-level regex for emoji annotation (compiled once)
static EMOJI_RE: OnceLock<Regex> = OnceLock::new();
/// Module-level regex for description annotation (compiled once)
static DESC_RE: OnceLock<Regex> = OnceLock::new();
/// Module-level regex for ignore annotation (compiled once)
static IGNORE_RE: OnceLock<Regex> = OnceLock::new();
/// Module-level regex for comment lines (compiled once)
static COMMENT_RE: OnceLock<Regex> = OnceLock::new();
/// Module-level regex for exported Go function definitions (compiled once)
static FUNC_RE: OnceLock<Regex> = OnceLock::new();

fn emoji_re() -> &'static Regex {
    EMOJI_RE.get_or_init(|| Regex::new(r"^\s*//\s*@emoji\s+(.+)$").expect("valid regex"))
}

fn desc_re() -> &'static Regex {
    DESC_RE.get_or_init(|| Regex::new(r"^\s*//\s*@description\s+(.+)$").expect("valid regex"))
}

fn ignore_re() -> &'static Regex {
    IGNORE_RE.get_or_init(|| Regex::new(r"^\s*//\s*@ignore\s*$").expect("valid regex"))
}

fn comment_re() -> &'static Regex {
    COMMENT_RE.get_or_init(|| Regex::new(r"^\s*//").expect("valid regex"))
}

fn func_re() -> &'static Regex {
    FUNC_RE.get_or_init(|| Regex::new(r"^func\s+([A-Z][a-zA-Z0-9_]*)\s*\(").expect("valid regex"))
}

/// Mage target item for TUI display
#[derive(Debug, Clone)]
pub struct MageTarget {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
}

/// Annotations extracted from magefile comments
#[derive(Debug, Clone, Default)]
pub struct MageAnnotations {
    pub emoji: Option<String>,
    pub description: Option<String>,
    pub ignored: bool,
}

/// Check if the `mage` binary is available.
pub fn is_mage_available() -> bool {
    *MAGE_AVAILABLE.get_or_init(|| {
        Command::new("mage")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Parse annotations from magefile comments.
pub fn parse_magefile_annotations(
    magefile_path: &Path,
) -> Result<HashMap<String, MageAnnotations>> {
    let content = fs::read_to_string(magefile_path)
        .with_context(|| format!("Failed to read magefile: {}", magefile_path.display()))?;
    parse_magefile_annotations_from_content(&content)
}

/// Parse annotations from magefile content (for testing).
///
/// Keys in the returned map are lowercased to match `mage -l` output which
/// lowercases the first letter of each target name.
pub fn parse_magefile_annotations_from_content(
    content: &str,
) -> Result<HashMap<String, MageAnnotations>> {
    let mut annotations_map: HashMap<String, MageAnnotations> = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        if line.starts_with(' ')
            || line.starts_with('\t')
            || line.trim().is_empty()
            || comment_re().is_match(line)
        {
            continue;
        }

        if let Some(cap) = func_re().captures(line) {
            let target_name = &cap[1];
            let mut emoji: Option<String> = None;
            let mut description: Option<String> = None;
            let mut ignored = false;

            let mut check_idx = line_idx.saturating_sub(1);
            loop {
                if check_idx >= line_idx {
                    break;
                }
                let prev_line = lines[check_idx];
                if !prev_line.trim().is_empty() && !comment_re().is_match(prev_line) {
                    break;
                }
                if ignore_re().is_match(prev_line) {
                    ignored = true;
                }
                if let Some(emoji_cap) = emoji_re().captures(prev_line) {
                    emoji = Some(emoji_cap[1].trim().to_string());
                }
                if let Some(desc_cap) = desc_re().captures(prev_line) {
                    description = Some(desc_cap[1].trim().to_string());
                }
                if check_idx == 0 {
                    break;
                }
                check_idx -= 1;
            }

            if emoji.is_some() || description.is_some() || ignored {
                // Lowercase the key so it matches the `mage -l` output which
                // lowercases the first character of each target (e.g. `Build` → `build`).
                let key = {
                    let mut chars = target_name.chars();
                    match chars.next() {
                        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                };
                annotations_map.insert(
                    key,
                    MageAnnotations {
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

/// Parse output from `mage -l` to extract target names and descriptions.
pub fn parse_mage_list_output(
    output: &str,
    category: &str,
    annotations: Option<&HashMap<String, MageAnnotations>>,
) -> Result<Vec<MageTarget>> {
    let mut targets = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        // Skip the "Targets:" header line and blank lines
        if line.is_empty() || line.starts_with("Targets") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        // Strip trailing `*` which mage appends to the default target
        let target_name = if let Some(first) = parts.first() {
            first.trim_end_matches('*').to_string()
        } else {
            continue;
        };

        let description_from_output = if parts.len() > 1 {
            parts[1..].join(" ")
        } else {
            String::new()
        };
        let target_annotations = annotations.and_then(|a| a.get(&target_name));
        let display_name = format_display_name(&target_name);

        let description = target_annotations
            .and_then(|a| a.description.clone())
            .or({
                if description_from_output.is_empty() {
                    None
                } else {
                    Some(description_from_output)
                }
            })
            .unwrap_or_else(|| format!("mage target {}", target_name));

        let emoji = target_annotations.and_then(|a| a.emoji.clone());
        let ignored = target_annotations.is_some_and(|a| a.ignored);

        targets.push(MageTarget {
            name: target_name,
            display_name,
            category: category.to_string(),
            description,
            emoji,
            ignored,
        });
    }
    targets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(targets)
}

/// Run `mage -l` and parse the result.
pub fn list_targets(magefile_path: &Path, category: &str) -> Result<Vec<MageTarget>> {
    let annotations = parse_magefile_annotations(magefile_path).ok();
    let magefile_dir = magefile_path
        .parent()
        .context("Failed to get parent directory of magefile")?;

    let output = Command::new("mage")
        .arg("-l")
        .current_dir(magefile_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to run mage for: {}", magefile_path.display()))?;

    if !output.status.success() {
        anyhow::bail!(
            "mage -l failed for {}: {}",
            magefile_path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    parse_mage_list_output(&output_str, category, annotations.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_mage_list_output ────────────────────────────────────────────────

    #[test]
    fn test_parse_mage_list_basic() {
        let output = "Targets:\n  build    Compile the binary\n  test     Run tests\n";
        let result = parse_mage_list_output(output, "mage", None).unwrap();
        assert_eq!(result.len(), 2);
        // sorted alphabetically
        assert_eq!(result[0].name, "build");
        assert_eq!(result[1].name, "test");
    }

    #[test]
    fn test_parse_mage_list_strips_default_star() {
        // `mage -l` marks the default target with a trailing `*`
        let output = "Targets:\n  build*   Compile the binary\n  test     Run tests\n";
        let result = parse_mage_list_output(output, "mage", None).unwrap();
        assert_eq!(result.len(), 2);
        // the `*` must be stripped
        assert_eq!(result[0].name, "build");
        assert_eq!(result[1].name, "test");
    }

    #[test]
    fn test_parse_mage_list_description_from_output() {
        let output = "Targets:\n  build    Compile the binary\n";
        let result = parse_mage_list_output(output, "mage", None).unwrap();
        assert_eq!(result[0].description, "Compile the binary");
    }

    #[test]
    fn test_parse_mage_list_fallback_description() {
        // No description in CLI output → fallback to "mage target <name>"
        let output = "Targets:\n  build\n";
        let result = parse_mage_list_output(output, "mage", None).unwrap();
        assert_eq!(result[0].description, "mage target build");
    }

    #[test]
    fn test_parse_mage_list_empty_output() {
        let output = "";
        let result = parse_mage_list_output(output, "mage", None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_mage_list_only_header() {
        let output = "Targets:\n";
        let result = parse_mage_list_output(output, "mage", None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_mage_list_annotation_overrides_description() {
        let output = "Targets:\n  build    CLI description\n";
        let mut annotations = HashMap::new();
        annotations.insert(
            "build".to_string(),
            MageAnnotations {
                description: Some("Annotation description".to_string()),
                ..Default::default()
            },
        );
        let result = parse_mage_list_output(output, "mage", Some(&annotations)).unwrap();
        assert_eq!(result[0].description, "Annotation description");
    }

    #[test]
    fn test_parse_mage_list_annotation_emoji() {
        let output = "Targets:\n  build    Compile the binary\n";
        let mut annotations = HashMap::new();
        annotations.insert(
            "build".to_string(),
            MageAnnotations {
                emoji: Some("🔨".to_string()),
                ..Default::default()
            },
        );
        let result = parse_mage_list_output(output, "mage", Some(&annotations)).unwrap();
        assert_eq!(result[0].emoji, Some("🔨".to_string()));
    }

    #[test]
    fn test_parse_mage_list_annotation_ignored() {
        let output = "Targets:\n  build    Compile the binary\n";
        let mut annotations = HashMap::new();
        annotations.insert(
            "build".to_string(),
            MageAnnotations {
                ignored: true,
                ..Default::default()
            },
        );
        let result = parse_mage_list_output(output, "mage", Some(&annotations)).unwrap();
        assert!(result[0].ignored);
    }

    // ── parse_magefile_annotations_from_content ───────────────────────────────

    #[test]
    fn test_parse_annotations_emoji() {
        let content = "// @emoji 🔨\nfunc Build(mg.Deps) {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        // key is lowercased
        assert!(annotations.contains_key("build"));
        assert_eq!(annotations["build"].emoji, Some("🔨".to_string()));
    }

    #[test]
    fn test_parse_annotations_description() {
        let content = "// @description Compile the binary\nfunc Build(mg.Deps) {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("build"));
        assert_eq!(
            annotations["build"].description,
            Some("Compile the binary".to_string())
        );
    }

    #[test]
    fn test_parse_annotations_ignore() {
        let content = "// @ignore\nfunc Helper() {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("helper"));
        assert!(annotations["helper"].ignored);
    }

    #[test]
    fn test_parse_annotations_all_three() {
        let content =
            "// @emoji 🚀\n// @description Deploy to production\n// @ignore\nfunc Deploy() {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert!(annotations.contains_key("deploy"));
        assert_eq!(annotations["deploy"].emoji, Some("🚀".to_string()));
        assert_eq!(
            annotations["deploy"].description,
            Some("Deploy to production".to_string())
        );
        assert!(annotations["deploy"].ignored);
    }

    #[test]
    fn test_parse_annotations_no_annotations() {
        let content = "func Build(mg.Deps) {\n}\nfunc Test() {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert!(!annotations.contains_key("build"));
        assert!(!annotations.contains_key("test"));
    }

    #[test]
    fn test_parse_annotations_lowercase_key_matches_mage_output() {
        // PascalCase Go func `RunTests` → mage -l outputs `runTests`
        let content = "// @emoji 🧪\nfunc RunTests() {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert!(
            annotations.contains_key("runTests"),
            "key should be camelCase to match mage -l"
        );
        assert_eq!(annotations["runTests"].emoji, Some("🧪".to_string()));
    }

    #[test]
    fn test_parse_annotations_skips_unexported_funcs() {
        // unexported (lowercase) functions are not mage targets
        let content = "// @emoji 🔨\nfunc helper() {\n}\n";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert!(annotations.is_empty());
    }

    #[test]
    fn test_parse_annotations_multiple_targets() {
        let content = "\
// @emoji 🔨
func Build() {}

// @emoji 🧪
func Test() {}

// @ignore
func Internal() {}
";
        let annotations = parse_magefile_annotations_from_content(content).unwrap();
        assert_eq!(annotations["build"].emoji, Some("🔨".to_string()));
        assert_eq!(annotations["test"].emoji, Some("🧪".to_string()));
        assert!(annotations["internal"].ignored);
    }

    // ── is_mage_available ─────────────────────────────────────────────────────

    #[test]
    fn test_is_mage_available_returns_bool() {
        // We can't control whether mage is installed, but we can ensure the
        // function returns without panicking and gives a consistent answer.
        let first = is_mage_available();
        let second = is_mage_available();
        assert_eq!(
            first, second,
            "OnceLock should return the same cached value"
        );
    }
}
