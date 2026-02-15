//! # Terraform / `OpenTofu` Parser
//!
//! This module discovers Terraform workspaces and provides common Terraform
//! commands for execution through the TUI.
//!
//! ## Overview
//!
//! When `*.tf` files are detected in a directory and either the `terraform` or
//! `tofu` (`OpenTofu`) binary is available, this parser provides:
//!
//! 1. **Common commands** — `init`, `plan`, `apply`, `destroy`, `validate`, `fmt`
//! 2. **Workspace commands** — `workspace select <name>` for each workspace
//!    discovered via `terraform workspace list` (or `tofu workspace list`)
//! 3. **Targeted commands** — `plan --target=<addr>`, `apply --target=<addr>`,
//!    and `destroy --target=<addr>` for each `resource`, `module`, and `data`
//!    block found in `.tf` files
//!
//! ## Binary Resolution
//!
//! The parser checks for tool availability in this order:
//! 1. `terraform` — `HashiCorp` Terraform
//! 2. `tofu` — `OpenTofu` (drop-in replacement)
//!
//! The first available binary is cached and used for all subsequent operations.
//! Use [`resolve_terraform_binary`] to get the resolved binary name.
//!
//! ## Key Types
//!
//! - [`TerraformCommand`] — Represents a Terraform command with display metadata
//! - [`TerraformCommandType`] — Distinguishes between common, workspace, and
//!   targeted commands
//! - [`is_terraform_available`] — Checks if `terraform` or `tofu` is installed
//! - [`resolve_terraform_binary`] — Returns the resolved binary name
//! - [`list_commands`] — Main entry point to list all Terraform commands
//! - [`parse_tf_resource_addresses`] — Extract resource addresses from `.tf` content
//! - [`discover_resource_addresses`] — Scan a directory for targetable resources
//!
//! ## CLI Integration
//!
//! Workspaces are discovered by running:
//! ```bash
//! terraform workspace list  # or: tofu workspace list
//! ```
//!
//! If the workspace directory is not initialized (no `.terraform/`), the parser
//! still returns common commands but skips workspace discovery.
//!
//! ## Execution
//!
//! Commands are executed based on their type:
//! - Common commands: `<binary> <command>` (e.g., `terraform plan` or `tofu plan`)
//! - Workspace selection: `<binary> workspace select <name>`
//! - Targeted commands: `<binary> <cmd> --target=<addr>` (e.g., `terraform apply --target=aws_instance.web`)
//!
//! ## Availability Caching
//!
//! The resolved binary name is cached using [`OnceLock`] to avoid repeated
//! process spawning during discovery.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};

use crate::script::discovery::format_display_name;

/// Cache for resolved terraform/tofu binary name (checked once per process).
/// Contains `Some("terraform")` or `Some("tofu")` if available, `None` otherwise.
static TERRAFORM_BINARY: OnceLock<Option<&'static str>> = OnceLock::new();

/// The type of Terraform command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerraformCommandType {
    /// A common Terraform command (init, plan, apply, etc.)
    Common,
    /// A workspace selection command (`terraform workspace select <name>`)
    Workspace,
    /// A targeted command (`terraform plan --target=<addr>`, etc.)
    Targeted,
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

/// Try a single binary name to see if it responds to `--version`.
fn check_binary(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolve which binary to use: `terraform` first, then `tofu`.
///
/// The result is cached for the lifetime of the process.
fn resolve_binary() -> Option<&'static str> {
    *TERRAFORM_BINARY.get_or_init(|| {
        if check_binary("terraform") {
            Some("terraform")
        } else if check_binary("tofu") {
            Some("tofu")
        } else {
            None
        }
    })
}

/// Check if either `terraform` or `tofu` is available.
pub fn is_terraform_available() -> bool {
    resolve_binary().is_some()
}

/// Return the resolved binary name (`"terraform"` or `"tofu"`).
///
/// Returns `None` if neither is installed.
pub fn resolve_terraform_binary() -> Option<&'static str> {
    resolve_binary()
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

/// Commands that support `--target` for resource-level operations.
const TARGETABLE_COMMANDS: &[(&str, &str)] = &[
    ("plan", "Plan changes for"),
    ("apply", "Apply changes to"),
    ("destroy", "Destroy"),
];

/// Parse a single `.tf` file's contents and extract resource addresses.
///
/// Recognises three block types:
/// - `resource "type" "name"` → `type.name`
/// - `module "name"` → `module.name`
/// - `data "type" "name"` → `data.type.name`
///
/// Only top-level blocks are matched (the keyword must appear at the start of a
/// line, ignoring leading whitespace). Quoted strings are expected around each
/// identifier.
pub fn parse_tf_resource_addresses(content: &str) -> Vec<String> {
    let mut addresses = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("resource ") {
            // resource "type" "name"
            if let Some(addr) = parse_two_labels(rest) {
                addresses.push(addr);
            }
        } else if let Some(rest) = trimmed.strip_prefix("module ") {
            // module "name"
            if let Some(name) = parse_one_label(rest) {
                addresses.push(format!("module.{name}"));
            }
        } else if let Some(rest) = trimmed.strip_prefix("data ") {
            // data "type" "name"
            if let Some(addr) = parse_two_labels(rest) {
                addresses.push(format!("data.{addr}"));
            }
        }
    }

    addresses
}

/// Extract two quoted labels from a string like `"aws_instance" "web" {`
/// and return `"aws_instance.web"`.
fn parse_two_labels(s: &str) -> Option<String> {
    let first_end = parse_one_label(s)?;
    // Skip past the first quoted label to find the second one
    let after_first = s.find('"')? + 1; // start of first label content
    let close_first = after_first + s[after_first..].find('"')?; // end of first label
    let rest = &s[close_first + 1..];
    let second = parse_one_label(rest)?;
    Some(format!("{first_end}.{second}"))
}

/// Extract a single quoted label from a string like `"web" {` → `"web"`.
fn parse_one_label(s: &str) -> Option<String> {
    let start = s.find('"')? + 1;
    let end = start + s[start..].find('"')?;
    let label = &s[start..end];
    if label.is_empty() {
        return None;
    }
    Some(label.to_string())
}

/// Scan a directory for `.tf` files and collect all resource addresses.
pub fn discover_resource_addresses(tf_dir: &Path) -> Vec<String> {
    let mut addresses = Vec::new();

    let entries = match fs::read_dir(tf_dir) {
        Ok(entries) => entries,
        Err(_) => return addresses,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("tf") {
            if let Ok(content) = fs::read_to_string(&path) {
                addresses.extend(parse_tf_resource_addresses(&content));
            }
        }
    }

    addresses.sort();
    addresses.dedup();
    addresses
}

/// Build targeted commands (`plan --target=...`, `apply --target=...`, etc.)
/// for each resource address.
fn build_targeted_commands(addresses: &[String], category: &str) -> Vec<TerraformCommand> {
    let mut commands = Vec::new();

    for addr in addresses {
        for (cmd, verb) in TARGETABLE_COMMANDS {
            commands.push(TerraformCommand {
                name: format!("{cmd} --target={addr}"),
                display_name: format!("{} --target={}", format_display_name(cmd), addr),
                category: category.to_string(),
                description: format!("{verb} {addr}"),
                emoji: Some("\u{1f3af}".to_string()), // 🎯
                ignored: false,
                command_type: TerraformCommandType::Targeted,
            });
        }
    }

    commands
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
            description: format!("Switch to the '{}' workspace", ws),
            emoji: Some("\u{1f4c2}".to_string()), // 📂
            ignored: false,
            command_type: TerraformCommandType::Workspace,
        })
        .collect()
}

/// Parse workspace list output, combine with common commands, and add targeted
/// commands for discovered resources.
///
/// This is the testable core — it takes the raw `terraform workspace list`
/// output (or `None` if workspace listing failed/was skipped) and a list of
/// resource addresses, then produces the full list of [`TerraformCommand`]s.
pub fn parse_terraform_commands(
    workspace_output: Option<&str>,
    resource_addresses: &[String],
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

    if !resource_addresses.is_empty() {
        commands.extend(build_targeted_commands(resource_addresses, category));
    }

    commands
}

/// Discover Terraform commands for a directory containing `.tf` files.
///
/// This runs `<binary> workspace list` (where `<binary>` is `terraform` or
/// `tofu`) to discover workspaces (if the directory has been initialized),
/// scans `.tf` files for resource/module/data blocks to generate targeted
/// commands, then combines them with the standard set of common commands.
pub fn list_commands(tf_dir: &Path, category: &str) -> Result<Vec<TerraformCommand>> {
    let binary = resolve_binary().context("Neither 'terraform' nor 'tofu' binary is available")?;

    // Try to list workspaces — this may fail if `terraform init` hasn't been run
    let workspace_output = Command::new(binary)
        .arg("workspace")
        .arg("list")
        .current_dir(tf_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Failed to run {} workspace list in: {}",
                binary,
                tf_dir.display()
            )
        })?;

    let ws_str = if workspace_output.status.success() {
        Some(String::from_utf8(workspace_output.stdout).unwrap_or_default())
    } else {
        None
    };

    // Discover resource addresses from .tf files
    let addresses = discover_resource_addresses(tf_dir);

    Ok(parse_terraform_commands(
        ws_str.as_deref(),
        &addresses,
        category,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_terraform_available / resolve_terraform_binary ---

    #[test]
    fn test_is_terraform_available_returns_bool() {
        // Just verify it doesn't panic — result depends on CI environment
        let _result = is_terraform_available();
    }

    #[test]
    fn test_resolve_terraform_binary_consistency() {
        // If available, the binary name should be "terraform" or "tofu"
        if let Some(binary) = resolve_terraform_binary() {
            assert!(
                binary == "terraform" || binary == "tofu",
                "Expected 'terraform' or 'tofu', got '{}'",
                binary
            );
        }
        // is_terraform_available should agree with resolve_terraform_binary
        assert_eq!(
            is_terraform_available(),
            resolve_terraform_binary().is_some()
        );
    }

    #[test]
    fn test_check_binary_nonexistent() {
        assert!(!check_binary("nonexistent_binary_xyz_12345"));
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
        assert_eq!(commands[0].description, "Switch to the 'default' workspace");
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

    // --- parse_terraform_commands (integration of common + workspace + targeted) ---

    #[test]
    fn test_parse_terraform_commands_no_workspaces() {
        let commands = parse_terraform_commands(None, &[], "myproject");
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
        let commands = parse_terraform_commands(Some(ws_output), &[], "myproject");
        assert_eq!(commands.len(), 6); // only common commands
    }

    #[test]
    fn test_parse_terraform_commands_multiple_workspaces() {
        let ws_output = "  default\n* staging\n  production\n";
        let commands = parse_terraform_commands(Some(ws_output), &[], "myproject");

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
        let commands = parse_terraform_commands(Some(""), &[], "myproject");
        // Empty output => no workspaces => only common commands
        assert_eq!(commands.len(), 6);
    }

    #[test]
    fn test_parse_terraform_commands_two_workspaces() {
        let ws_output = "* default\n  staging\n";
        let commands = parse_terraform_commands(Some(ws_output), &[], "infra");

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

    #[test]
    fn test_parse_terraform_commands_with_resources() {
        let addrs = vec!["aws_instance.web".to_string(), "module.vpc".to_string()];
        let commands = parse_terraform_commands(None, &addrs, "infra");

        // 6 common + 2 resources * 3 targetable commands = 12
        assert_eq!(commands.len(), 12);

        let targeted: Vec<_> = commands
            .iter()
            .filter(|c| c.command_type == TerraformCommandType::Targeted)
            .collect();
        assert_eq!(targeted.len(), 6);
    }

    #[test]
    fn test_parse_terraform_commands_workspaces_and_resources() {
        let ws_output = "  default\n* staging\n";
        let addrs = vec!["local_file.hello".to_string()];
        let commands = parse_terraform_commands(Some(ws_output), &addrs, "myproject");

        // 6 common + 2 workspace + 1 resource * 3 targeted = 11
        assert_eq!(commands.len(), 11);
    }

    // --- parse_tf_resource_addresses ---

    #[test]
    fn test_parse_tf_resource_addresses_resource() {
        let content = r#"
resource "aws_instance" "web" {
  ami           = "ami-123456"
  instance_type = "t2.micro"
}
"#;
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(addrs, vec!["aws_instance.web"]);
    }

    #[test]
    fn test_parse_tf_resource_addresses_module() {
        let content = r#"
module "vpc" {
  source = "./modules/vpc"
}
"#;
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(addrs, vec!["module.vpc"]);
    }

    #[test]
    fn test_parse_tf_resource_addresses_data() {
        let content = r#"
data "aws_ami" "latest" {
  most_recent = true
}
"#;
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(addrs, vec!["data.aws_ami.latest"]);
    }

    #[test]
    fn test_parse_tf_resource_addresses_multiple() {
        let content = r#"
resource "local_file" "hello" {
  content  = "hello"
  filename = "hello.txt"
}

resource "null_resource" "echo" {
  provisioner "local-exec" {
    command = "echo hello"
  }
}

module "networking" {
  source = "./networking"
}

data "aws_caller_identity" "current" {}
"#;
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(
            addrs,
            vec![
                "local_file.hello",
                "null_resource.echo",
                "module.networking",
                "data.aws_caller_identity.current",
            ]
        );
    }

    #[test]
    fn test_parse_tf_resource_addresses_empty() {
        let content = r#"
terraform {
  required_version = ">= 1.0"
}

variable "name" {
  type = string
}

output "result" {
  value = var.name
}
"#;
        let addrs = parse_tf_resource_addresses(content);
        assert!(addrs.is_empty());
    }

    #[test]
    fn test_parse_tf_resource_addresses_indented() {
        // Indented blocks should still be matched (trim handles this)
        let content = "  resource \"aws_s3_bucket\" \"my_bucket\" {\n  }\n";
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(addrs, vec!["aws_s3_bucket.my_bucket"]);
    }

    #[test]
    fn test_parse_tf_resource_addresses_no_braces_on_same_line() {
        // Opening brace on next line
        let content = "resource \"aws_instance\" \"main\"\n{\n  ami = \"abc\"\n}\n";
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(addrs, vec!["aws_instance.main"]);
    }

    #[test]
    fn test_parse_tf_resource_addresses_comments_ignored() {
        // A comment containing "resource" should not match (it won't start with resource after trim)
        let content = "# resource \"fake\" \"thing\" {}\nresource \"real\" \"item\" {\n}\n";
        let addrs = parse_tf_resource_addresses(content);
        assert_eq!(addrs, vec!["real.item"]);
    }

    #[test]
    fn test_parse_tf_resource_addresses_empty_labels() {
        // Empty quoted labels should be skipped
        let content = "resource \"\" \"name\" {}\n";
        let addrs = parse_tf_resource_addresses(content);
        assert!(addrs.is_empty());
    }

    // --- parse_one_label / parse_two_labels ---

    #[test]
    fn test_parse_one_label() {
        assert_eq!(parse_one_label("\"hello\" {"), Some("hello".to_string()));
        assert_eq!(parse_one_label("\"vpc\""), Some("vpc".to_string()));
        assert_eq!(parse_one_label("no quotes"), None);
        assert_eq!(parse_one_label("\"\" {}"), None);
    }

    #[test]
    fn test_parse_two_labels() {
        assert_eq!(
            parse_two_labels("\"aws_instance\" \"web\" {"),
            Some("aws_instance.web".to_string())
        );
        assert_eq!(parse_two_labels("\"only_one\""), None);
        assert_eq!(parse_two_labels("no quotes at all"), None);
    }

    // --- build_targeted_commands ---

    #[test]
    fn test_build_targeted_commands() {
        let addrs = vec!["aws_instance.web".to_string()];
        let commands = build_targeted_commands(&addrs, "infra");

        assert_eq!(commands.len(), 3);

        assert_eq!(commands[0].name, "plan --target=aws_instance.web");
        assert_eq!(commands[0].display_name, "Plan --target=aws_instance.web");
        assert_eq!(commands[0].description, "Plan changes for aws_instance.web");
        assert_eq!(commands[0].command_type, TerraformCommandType::Targeted);
        assert_eq!(commands[0].emoji, Some("\u{1f3af}".to_string())); // 🎯

        assert_eq!(commands[1].name, "apply --target=aws_instance.web");
        assert_eq!(commands[2].name, "destroy --target=aws_instance.web");
    }

    #[test]
    fn test_build_targeted_commands_multiple_addresses() {
        let addrs = vec!["local_file.hello".to_string(), "module.vpc".to_string()];
        let commands = build_targeted_commands(&addrs, "infra");

        // 2 addresses * 3 targetable commands = 6
        assert_eq!(commands.len(), 6);

        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"plan --target=local_file.hello"));
        assert!(names.contains(&"apply --target=local_file.hello"));
        assert!(names.contains(&"destroy --target=local_file.hello"));
        assert!(names.contains(&"plan --target=module.vpc"));
        assert!(names.contains(&"apply --target=module.vpc"));
        assert!(names.contains(&"destroy --target=module.vpc"));
    }

    #[test]
    fn test_build_targeted_commands_empty() {
        let commands = build_targeted_commands(&[], "infra");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_build_targeted_commands_category() {
        let addrs = vec!["null_resource.test".to_string()];
        let commands = build_targeted_commands(&addrs, "my-infra");
        for cmd in &commands {
            assert_eq!(cmd.category, "my-infra");
            assert!(!cmd.ignored);
        }
    }

    // --- TerraformCommandType ---

    #[test]
    fn test_terraform_command_type_equality() {
        assert_eq!(TerraformCommandType::Common, TerraformCommandType::Common);
        assert_eq!(
            TerraformCommandType::Workspace,
            TerraformCommandType::Workspace
        );
        assert_eq!(
            TerraformCommandType::Targeted,
            TerraformCommandType::Targeted
        );
        assert_ne!(
            TerraformCommandType::Common,
            TerraformCommandType::Workspace
        );
        assert_ne!(TerraformCommandType::Common, TerraformCommandType::Targeted);
        assert_ne!(
            TerraformCommandType::Workspace,
            TerraformCommandType::Targeted
        );
    }
}
