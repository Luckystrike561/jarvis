//! # Terraform Parser
//!
//! This module discovers Terraform workspaces and provides common Terraform
//! commands for execution through the TUI.
//!
//! ## Overview
//!
//! When `*.tf` files are detected in a directory and the `terraform` binary is
//! available, this parser provides:
//!
//! 1. **Common commands** — `init`, `plan`, `apply`, `destroy`, `validate`, `fmt`
//! 2. **Workspace commands** — `workspace select <name>` for each workspace
//!    discovered via `terraform workspace list`
//!
//! ## Key Types
//!
//! - [`TerraformCommand`] — Represents a Terraform command with display metadata
//! - [`TerraformCommandType`] — Distinguishes between common commands and
//!   workspace commands
//! - [`is_terraform_available`] — Checks if the `terraform` CLI is installed
//! - [`list_commands`] — Main entry point to list all Terraform commands
//!
//! ## CLI Integration
//!
//! Workspaces are discovered by running:
//! ```bash
//! terraform workspace list
//! ```
//!
//! If the workspace directory is not initialized (no `.terraform/`), the parser
//! still returns common commands but skips workspace discovery.
//!
//! ## Execution
//!
//! Commands are executed based on their type:
//! - Common commands: `terraform <command>` (e.g., `terraform plan`)
//! - Workspace selection: `terraform workspace select <name>`
//!
//! ## Availability Caching
//!
//! The `terraform` binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::script::discovery::format_display_name;

/// Cache for terraform availability check (checked once per process)
static TERRAFORM_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// The type of Terraform command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerraformCommandType {
    /// A common Terraform command (init, plan, apply, etc.)
    Common,
    /// A workspace selection command (`terraform workspace select <name>`)
    Workspace,
}

/// Terraform command item for TUI display
#[derive(Debug, Clone)]
pub struct TerraformCommand {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
    pub command_type: TerraformCommandType,
}

/// Check if the `terraform` binary is available.
pub fn is_terraform_available() -> bool {
    *TERRAFORM_AVAILABLE.get_or_init(|| {
        Command::new("terraform")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// The set of common Terraform commands that are always available.
const COMMON_COMMANDS: &[(&str, &str)] = &[
    ("init", "Initialize a Terraform working directory"),
    ("validate", "Validate the Terraform configuration files"),
    ("plan", "Show an execution plan for infrastructure changes"),
    ("apply", "Apply the planned infrastructure changes"),
    ("destroy", "Destroy all managed infrastructure"),
    ("fmt", "Format Terraform configuration files"),
];

/// Build the list of common Terraform commands for a given category.
fn build_common_commands(category: &str) -> Vec<TerraformCommand> {
    COMMON_COMMANDS
        .iter()
        .map(|(name, desc)| TerraformCommand {
            name: (*name).to_string(),
            display_name: format_display_name(name),
            category: category.to_string(),
            description: (*desc).to_string(),
            emoji: Some("\u{1f3d7}\u{fe0f}".to_string()), // 🏗️
            ignored: false,
            command_type: TerraformCommandType::Common,
        })
        .collect()
}

/// Parse the output of `terraform workspace list`.
///
/// The output format is one workspace per line, with the active workspace
/// prefixed by `* `. For example:
///
/// ```text
///   default
/// * staging
///   production
/// ```
///
/// Returns a list of workspace names (without the `*` prefix and leading
/// whitespace).
fn parse_workspace_list(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Strip the "* " prefix from the active workspace
            let name = trimmed.strip_prefix("* ").unwrap_or(trimmed);
            Some(name.to_string())
        })
        .collect()
}

/// Build workspace selection commands from parsed workspace names.
fn build_workspace_commands(workspaces: &[String], category: &str) -> Vec<TerraformCommand> {
    workspaces
        .iter()
        .map(|ws| TerraformCommand {
            name: format!("workspace select {}", ws),
            display_name: format!("Workspace: {}", format_display_name(ws)),
            category: category.to_string(),
            description: format!("terraform workspace select {}", ws),
            emoji: Some("\u{1f4c2}".to_string()), // 📂
            ignored: false,
            command_type: TerraformCommandType::Workspace,
        })
        .collect()
}

/// Parse workspace list output and combine with common commands.
///
/// This is the testable core — it takes the raw `terraform workspace list`
/// output (or `None` if workspace listing failed/was skipped) and produces
/// the full list of [`TerraformCommand`]s.
pub fn parse_terraform_commands(
    workspace_output: Option<&str>,
    category: &str,
) -> Vec<TerraformCommand> {
    let mut commands = build_common_commands(category);

    if let Some(output) = workspace_output {
        let workspaces = parse_workspace_list(output);
        // Only add workspace commands if there are multiple workspaces
        // (a single "default" workspace is not useful to switch to)
        if workspaces.len() > 1 {
            commands.extend(build_workspace_commands(&workspaces, category));
        }
    }

    commands
}

/// Discover Terraform commands for a directory containing `.tf` files.
///
/// This runs `terraform workspace list` to discover workspaces (if the
/// directory has been initialized), then combines them with the standard
/// set of common commands.
pub fn list_commands(tf_dir: &Path, category: &str) -> Result<Vec<TerraformCommand>> {
    // Try to list workspaces — this may fail if `terraform init` hasn't been run
    let workspace_output = Command::new("terraform")
        .arg("workspace")
        .arg("list")
        .current_dir(tf_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Failed to run terraform workspace list in: {}",
                tf_dir.display()
            )
        })?;

    let ws_str = if workspace_output.status.success() {
        Some(String::from_utf8(workspace_output.stdout).unwrap_or_default())
    } else {
        None
    };

    Ok(parse_terraform_commands(ws_str.as_deref(), category))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_terraform_available ---

    #[test]
    fn test_is_terraform_available_returns_bool() {
        // Just verify it doesn't panic — result depends on CI environment
        let _result = is_terraform_available();
    }

    // --- parse_workspace_list ---

    #[test]
    fn test_parse_workspace_list_single_default() {
        let output = "* default\n";
        let workspaces = parse_workspace_list(output);
        assert_eq!(workspaces, vec!["default"]);
    }

    #[test]
    fn test_parse_workspace_list_multiple() {
        let output = "  default\n* staging\n  production\n";
        let workspaces = parse_workspace_list(output);
        assert_eq!(workspaces, vec!["default", "staging", "production"]);
    }

    #[test]
    fn test_parse_workspace_list_with_extra_whitespace() {
        let output = "  default  \n  staging  \n";
        let workspaces = parse_workspace_list(output);
        // trim() will strip trailing whitespace, but workspace names shouldn't
        // have trailing spaces — terraform doesn't produce them. We trim for safety.
        assert_eq!(workspaces.len(), 2);
    }

    #[test]
    fn test_parse_workspace_list_empty() {
        let output = "";
        let workspaces = parse_workspace_list(output);
        assert!(workspaces.is_empty());
    }

    #[test]
    fn test_parse_workspace_list_blank_lines() {
        let output = "\n\n* default\n\n  staging\n\n";
        let workspaces = parse_workspace_list(output);
        assert_eq!(workspaces, vec!["default", "staging"]);
    }

    #[test]
    fn test_parse_workspace_list_complex_names() {
        let output = "  dev-us-east-1\n* prod-eu-west-1\n  staging-ap-south-1\n";
        let workspaces = parse_workspace_list(output);
        assert_eq!(
            workspaces,
            vec!["dev-us-east-1", "prod-eu-west-1", "staging-ap-south-1"]
        );
    }

    // --- build_common_commands ---

    #[test]
    fn test_build_common_commands_count() {
        let commands = build_common_commands("myproject");
        assert_eq!(commands.len(), 6);
    }

    #[test]
    fn test_build_common_commands_names() {
        let commands = build_common_commands("myproject");
        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["init", "validate", "plan", "apply", "destroy", "fmt"]
        );
    }

    #[test]
    fn test_build_common_commands_category() {
        let commands = build_common_commands("infra");
        for cmd in &commands {
            assert_eq!(cmd.category, "infra");
        }
    }

    #[test]
    fn test_build_common_commands_display_names() {
        let commands = build_common_commands("myproject");
        let init = commands.iter().find(|c| c.name == "init").expect("init");
        assert_eq!(init.display_name, "Init");

        let validate = commands
            .iter()
            .find(|c| c.name == "validate")
            .expect("validate");
        assert_eq!(validate.display_name, "Validate");
    }

    #[test]
    fn test_build_common_commands_descriptions() {
        let commands = build_common_commands("myproject");
        let plan = commands.iter().find(|c| c.name == "plan").expect("plan");
        assert!(plan.description.contains("execution plan"));

        let apply = commands.iter().find(|c| c.name == "apply").expect("apply");
        assert!(apply.description.contains("Apply"));
    }

    #[test]
    fn test_build_common_commands_emoji() {
        let commands = build_common_commands("myproject");
        for cmd in &commands {
            assert_eq!(cmd.emoji, Some("\u{1f3d7}\u{fe0f}".to_string()));
            assert_eq!(cmd.command_type, TerraformCommandType::Common);
        }
    }

    #[test]
    fn test_build_common_commands_not_ignored() {
        let commands = build_common_commands("myproject");
        for cmd in &commands {
            assert!(!cmd.ignored);
        }
    }

    // --- build_workspace_commands ---

    #[test]
    fn test_build_workspace_commands() {
        let workspaces = vec!["default".to_string(), "staging".to_string()];
        let commands = build_workspace_commands(&workspaces, "infra");

        assert_eq!(commands.len(), 2);

        assert_eq!(commands[0].name, "workspace select default");
        assert_eq!(commands[0].display_name, "Workspace: Default");
        assert_eq!(
            commands[0].description,
            "terraform workspace select default"
        );
        assert_eq!(commands[0].command_type, TerraformCommandType::Workspace);
        assert_eq!(commands[0].emoji, Some("\u{1f4c2}".to_string())); // 📂

        assert_eq!(commands[1].name, "workspace select staging");
        assert_eq!(commands[1].display_name, "Workspace: Staging");
    }

    #[test]
    fn test_build_workspace_commands_complex_names() {
        let workspaces = vec!["dev-us-east-1".to_string()];
        let commands = build_workspace_commands(&workspaces, "infra");

        assert_eq!(commands[0].name, "workspace select dev-us-east-1");
        assert_eq!(commands[0].display_name, "Workspace: Dev Us East 1");
    }

    #[test]
    fn test_build_workspace_commands_empty() {
        let commands = build_workspace_commands(&[], "infra");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_build_workspace_commands_category() {
        let workspaces = vec!["staging".to_string()];
        let commands = build_workspace_commands(&workspaces, "my-infra");
        assert_eq!(commands[0].category, "my-infra");
    }

    // --- parse_terraform_commands (integration of common + workspace) ---

    #[test]
    fn test_parse_terraform_commands_no_workspaces() {
        let commands = parse_terraform_commands(None, "myproject");
        // Should have only common commands
        assert_eq!(commands.len(), 6);
        for cmd in &commands {
            assert_eq!(cmd.command_type, TerraformCommandType::Common);
        }
    }

    #[test]
    fn test_parse_terraform_commands_single_default_workspace() {
        // A single "default" workspace shouldn't produce workspace commands
        let ws_output = "* default\n";
        let commands = parse_terraform_commands(Some(ws_output), "myproject");
        assert_eq!(commands.len(), 6); // only common commands
    }

    #[test]
    fn test_parse_terraform_commands_multiple_workspaces() {
        let ws_output = "  default\n* staging\n  production\n";
        let commands = parse_terraform_commands(Some(ws_output), "myproject");

        // 6 common + 3 workspace = 9
        assert_eq!(commands.len(), 9);

        let common: Vec<_> = commands
            .iter()
            .filter(|c| c.command_type == TerraformCommandType::Common)
            .collect();
        let workspace: Vec<_> = commands
            .iter()
            .filter(|c| c.command_type == TerraformCommandType::Workspace)
            .collect();

        assert_eq!(common.len(), 6);
        assert_eq!(workspace.len(), 3);
    }

    #[test]
    fn test_parse_terraform_commands_empty_workspace_output() {
        let commands = parse_terraform_commands(Some(""), "myproject");
        // Empty output => no workspaces => only common commands
        assert_eq!(commands.len(), 6);
    }

    #[test]
    fn test_parse_terraform_commands_two_workspaces() {
        let ws_output = "* default\n  staging\n";
        let commands = parse_terraform_commands(Some(ws_output), "infra");

        // 6 common + 2 workspace = 8
        assert_eq!(commands.len(), 8);

        let ws_cmds: Vec<_> = commands
            .iter()
            .filter(|c| c.command_type == TerraformCommandType::Workspace)
            .collect();
        assert_eq!(ws_cmds.len(), 2);

        let ws_names: Vec<&str> = ws_cmds.iter().map(|c| c.name.as_str()).collect();
        assert!(ws_names.contains(&"workspace select default"));
        assert!(ws_names.contains(&"workspace select staging"));
    }

    // --- TerraformCommandType ---

    #[test]
    fn test_terraform_command_type_equality() {
        assert_eq!(TerraformCommandType::Common, TerraformCommandType::Common);
        assert_eq!(
            TerraformCommandType::Workspace,
            TerraformCommandType::Workspace
        );
        assert_ne!(
            TerraformCommandType::Common,
            TerraformCommandType::Workspace
        );
    }
}
