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
pub fn parse_magefile_annotations_from_content(
    content: &str,
) -> Result<HashMap<String, MageAnnotations>> {
    let mut annotations_map: HashMap<String, MageAnnotations> = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();

    let emoji_re = Regex::new(r"^\s*//\s*@emoji\s+(.+)$").context("Failed to compile emoji regex pattern")?;
    let desc_re = Regex::new(r"^\s*//\s*@description\s+(.+)$").context("Failed to compile description regex pattern")?;
    let ignore_re = Regex::new(r"^\s*//\s*@ignore\s*$").context("Failed to compile ignore regex pattern")?;
    let comment_re = Regex::new(r"^\s*//").context("Failed to compile comment regex pattern")?;
    let func_re = Regex::new(r"^func\s+([A-Z][a-zA-Z0-9_]*)\s*\(").context("Failed to compile function regex pattern")?;

    for (line_idx, line) in lines.iter().enumerate() {
        if line.starts_with(' ') || line.starts_with('\t') || line.trim().is_empty() || comment_re.is_match(line) {
            continue;
        }

        if let Some(cap) = func_re.captures(line) {
            let target_name = &cap[1];
            let mut emoji: Option<String> = None;
            let mut description: Option<String> = None;
            let mut ignored = false;

            let mut check_idx = line_idx.saturating_sub(1);
            loop {
                if check_idx >= line_idx { break; }
                let prev_line = lines[check_idx];
                if !prev_line.trim().is_empty() && !comment_re.is_match(prev_line) { break; }
                if ignore_re.is_match(prev_line) { ignored = true; }
                if let Some(emoji_cap) = emoji_re.captures(prev_line) {
                    emoji = Some(emoji_cap[1].trim().to_string());
                }
                if let Some(desc_cap) = desc_re.captures(prev_line) {
                    description = Some(desc_cap[1].trim().to_string());
                }
                if check_idx == 0 { break; }
                check_idx -= 1;
            }

            if emoji.as_ref().is_some() || description.as_ref().is_some() || ignored {
                annotations_map.insert(
                    target_name.to_string(),
                    MageAnnotations { emoji, description, ignored },
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
        if line.is_empty() || line == "Targets:" { continue; }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        let target_name = if let Some(first) = parts.first() { first.to_string() } else { continue; };
        
        let description_from_output = if parts.len() > 1 { parts[1..].join(" ") } else { String::new() };
        let target_annotations = annotations.and_then(|a| a.get(&target_name));
        let display_name = format_display_name(&target_name);
        
        let description = target_annotations
            .and_then(|a| a.description.clone())
            .or_else(|| if description_from_output.is_empty() { None } else { Some(description_from_output) })
            .unwrap_or_else(|| format!("mage target {}", target_name));
        
        let emoji = target_annotations.and_then(|a| a.emoji.clone());
        let ignored = target_annotations.is_some_and(|a| a.ignored);

        targets.push(MageTarget { name: target_name, display_name, category: category.to_string(), description, emoji, ignored });
    }
    targets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(targets)
}

/// Run `mage -l` and parse the result.
pub fn list_targets(magefile_path: &Path, category: &str) -> Result<Vec<MageTarget>> {
    let annotations = parse_magefile_annotations(magefile_path).ok();
    let magefile_dir = magefile_path.parent().context("Failed to get parent directory of magefile")?;

    let output = Command::new("mage").arg("-l").current_dir(magefile_dir)
        .stdout(Stdio::piped()).stderr(Stdio::null()).output()
        .with_context(|| format!("Failed to run mage for: {}", magefile_path.display()))?;

    if !output.status.success() {
        anyhow::bail!("mage -l failed for {}: {}", magefile_path.display(), String::from_utf8_lossy(&output.stderr));
    }

    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    parse_mage_list_output(&output_str, category, annotations.as_ref())
}
