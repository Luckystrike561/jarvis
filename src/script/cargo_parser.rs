//! # Cargo (Rust) Parser
//!
//! This module parses `Cargo.toml` configurations and extracts available targets.
//!
//! ## Overview
//!
//! This parser invokes `cargo metadata` to get target information. This approach:
//!
//! - Handles complex workspace layouts
//! - Resolves all binary and example targets accurately
//! - Gets target names from the build system itself
//!
//! ## Key Types
//!
//! - [`CargoTarget`] - Represents a cargo target with display metadata for the TUI
//! - [`CargoTargetType`] - Distinguishes between binary and example targets
//! - [`is_cargo_available`] - Checks if `cargo` CLI is installed
//! - [`list_targets`] - Main function to list targets from a Cargo.toml
//!
//! ## CLI Integration
//!
//! The parser runs:
//! ```bash
//! cargo metadata --format-version 1 --no-deps --manifest-path <path>
//! ```
//!
//! And parses the JSON output to extract binary and example targets.
//!
//! ## Execution
//!
//! Targets are executed based on their type:
//! - Binary targets: `cargo run --bin <name> --manifest-path <path>`
//! - Example targets: `cargo run --example <name> --manifest-path <path>`
//!
//! ## Availability Caching
//!
//! The `cargo` binary availability is cached using [`OnceLock`] to avoid
//! repeated process spawning during discovery.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::script::discovery::format_display_name;

/// Cache for cargo availability check (checked once per process)
static CARGO_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// The type of cargo target (binary or example)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CargoTargetType {
    /// A binary target (`cargo run --bin <name>`)
    Binary,
    /// An example target (`cargo run --example <name>`)
    Example,
}

/// Cargo target item for TUI display (mirrors other script types)
#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
    pub target_type: CargoTargetType,
}

/// Check if the `cargo` binary is available.
pub fn is_cargo_available() -> bool {
    *CARGO_AVAILABLE.get_or_init(|| {
        Command::new("cargo")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Parse `cargo metadata` JSON output to extract binary and example targets.
///
/// The metadata output includes a `packages` array, each with a `targets` array.
/// We filter targets by kind: `"bin"` for binaries and `"example"` for examples.
fn parse_cargo_metadata(output: &str, category: &str) -> Result<Vec<CargoTarget>> {
    let metadata: Value =
        serde_json::from_str(output).context("Failed to parse cargo metadata JSON")?;

    let mut targets = Vec::new();

    let packages = metadata["packages"]
        .as_array()
        .context("Expected 'packages' array in cargo metadata")?;

    for package in packages {
        let package_targets = match package["targets"].as_array() {
            Some(t) => t,
            None => continue,
        };

        for target in package_targets {
            let name = match target["name"].as_str() {
                Some(n) => n.to_string(),
                None => continue,
            };

            let kinds = match target["kind"].as_array() {
                Some(k) => k,
                None => continue,
            };

            // Determine target type from kind
            let target_type = if kinds.iter().any(|k| k.as_str() == Some("bin")) {
                CargoTargetType::Binary
            } else if kinds.iter().any(|k| k.as_str() == Some("example")) {
                CargoTargetType::Example
            } else {
                // Skip lib, test, bench, custom-build, proc-macro targets
                continue;
            };

            let display_name = format_display_name(&name);

            let description = match target_type {
                CargoTargetType::Binary => format!("cargo run --bin {}", name),
                CargoTargetType::Example => format!("cargo run --example {}", name),
            };

            let emoji = match target_type {
                CargoTargetType::Binary => Some("\u{1f4e6}".to_string()), // 📦
                CargoTargetType::Example => Some("\u{1f4d6}".to_string()), // 📖
            };

            targets.push(CargoTarget {
                name,
                display_name,
                category: category.to_string(),
                description,
                emoji,
                ignored: false,
                target_type,
            });
        }
    }

    targets.sort_by(|a, b| {
        // Sort examples after binaries, then alphabetically by name
        a.target_type
            .cmp_order()
            .cmp(&b.target_type.cmp_order())
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(targets)
}

impl CargoTargetType {
    /// Returns a sort order value for grouping binaries before examples.
    fn cmp_order(self) -> u8 {
        match self {
            CargoTargetType::Binary => 0,
            CargoTargetType::Example => 1,
        }
    }
}

/// Run `cargo metadata` and parse the result.
///
/// Extracts binary and example targets from the Cargo.toml manifest.
pub fn list_targets(manifest_path: &Path, category: &str) -> Result<Vec<CargoTarget>> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .with_context(|| {
            format!(
                "Failed to run cargo metadata for: {}",
                manifest_path.display()
            )
        })?;

    if !output.status.success() {
        anyhow::bail!("cargo metadata failed for {}", manifest_path.display(),);
    }

    let output_str = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).to_string(),
    };

    parse_cargo_metadata(&output_str, category)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metadata() -> String {
        r#"{
            "packages": [{
                "name": "my-app",
                "version": "0.1.0",
                "targets": [
                    {
                        "name": "my-app",
                        "kind": ["bin"],
                        "src_path": "src/main.rs"
                    },
                    {
                        "name": "my-app",
                        "kind": ["lib"],
                        "src_path": "src/lib.rs"
                    },
                    {
                        "name": "server",
                        "kind": ["bin"],
                        "src_path": "src/bin/server.rs"
                    },
                    {
                        "name": "basic",
                        "kind": ["example"],
                        "src_path": "examples/basic.rs"
                    },
                    {
                        "name": "advanced",
                        "kind": ["example"],
                        "src_path": "examples/advanced.rs"
                    },
                    {
                        "name": "integration",
                        "kind": ["test"],
                        "src_path": "tests/integration.rs"
                    },
                    {
                        "name": "bench_perf",
                        "kind": ["bench"],
                        "src_path": "benches/bench_perf.rs"
                    },
                    {
                        "name": "build-script-build",
                        "kind": ["custom-build"],
                        "src_path": "build.rs"
                    }
                ]
            }],
            "workspace_members": ["my-app 0.1.0"]
        }"#
        .to_string()
    }

    #[test]
    fn test_parse_cargo_metadata_extracts_bins_and_examples() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "myproject").unwrap();

        // Should find: my-app (bin), server (bin), basic (example), advanced (example)
        // Should NOT find: my-app (lib), integration (test), bench_perf (bench), build-script-build
        assert_eq!(targets.len(), 4);

        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"my-app"));
        assert!(names.contains(&"server"));
        assert!(names.contains(&"basic"));
        assert!(names.contains(&"advanced"));
    }

    #[test]
    fn test_parse_cargo_metadata_target_types() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "myproject").unwrap();

        let bins: Vec<_> = targets
            .iter()
            .filter(|t| t.target_type == CargoTargetType::Binary)
            .collect();
        let examples: Vec<_> = targets
            .iter()
            .filter(|t| t.target_type == CargoTargetType::Example)
            .collect();

        assert_eq!(bins.len(), 2);
        assert_eq!(examples.len(), 2);
    }

    #[test]
    fn test_parse_cargo_metadata_sort_order() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "myproject").unwrap();

        // Binaries should come before examples, each sorted alphabetically
        assert_eq!(targets[0].name, "my-app");
        assert_eq!(targets[0].target_type, CargoTargetType::Binary);
        assert_eq!(targets[1].name, "server");
        assert_eq!(targets[1].target_type, CargoTargetType::Binary);
        assert_eq!(targets[2].name, "advanced");
        assert_eq!(targets[2].target_type, CargoTargetType::Example);
        assert_eq!(targets[3].name, "basic");
        assert_eq!(targets[3].target_type, CargoTargetType::Example);
    }

    #[test]
    fn test_parse_cargo_metadata_descriptions() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "myproject").unwrap();

        let bin = targets.iter().find(|t| t.name == "server").unwrap();
        assert_eq!(bin.description, "cargo run --bin server");

        let example = targets.iter().find(|t| t.name == "basic").unwrap();
        assert_eq!(example.description, "cargo run --example basic");
    }

    #[test]
    fn test_parse_cargo_metadata_category() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "my-project").unwrap();

        for target in &targets {
            assert_eq!(target.category, "my-project");
        }
    }

    #[test]
    fn test_parse_cargo_metadata_display_names() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "myproject").unwrap();

        let app = targets.iter().find(|t| t.name == "my-app").unwrap();
        assert_eq!(app.display_name, "My App");

        let server = targets.iter().find(|t| t.name == "server").unwrap();
        assert_eq!(server.display_name, "Server");
    }

    #[test]
    fn test_parse_cargo_metadata_emojis() {
        let metadata = sample_metadata();
        let targets = parse_cargo_metadata(&metadata, "myproject").unwrap();

        let bin = targets.iter().find(|t| t.name == "server").unwrap();
        assert_eq!(bin.emoji, Some("\u{1f4e6}".to_string())); // 📦

        let example = targets.iter().find(|t| t.name == "basic").unwrap();
        assert_eq!(example.emoji, Some("\u{1f4d6}".to_string())); // 📖
    }

    #[test]
    fn test_parse_cargo_metadata_empty_packages() {
        let metadata = r#"{"packages": [], "workspace_members": []}"#;
        let targets = parse_cargo_metadata(metadata, "myproject").unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_cargo_metadata_no_targets() {
        let metadata = r#"{"packages": [{"name": "empty", "version": "0.1.0", "targets": []}], "workspace_members": []}"#;
        let targets = parse_cargo_metadata(metadata, "myproject").unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_cargo_metadata_lib_only() {
        let metadata = r#"{
            "packages": [{
                "name": "mylib",
                "version": "0.1.0",
                "targets": [
                    {"name": "mylib", "kind": ["lib"], "src_path": "src/lib.rs"}
                ]
            }],
            "workspace_members": []
        }"#;
        let targets = parse_cargo_metadata(metadata, "myproject").unwrap();
        // Library targets should be skipped
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_cargo_metadata_invalid_json() {
        let result = parse_cargo_metadata("not json", "myproject");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cargo_metadata_workspace() {
        let metadata = r#"{
            "packages": [
                {
                    "name": "crate-a",
                    "version": "0.1.0",
                    "targets": [
                        {"name": "crate-a", "kind": ["bin"], "src_path": "crates/a/src/main.rs"}
                    ]
                },
                {
                    "name": "crate-b",
                    "version": "0.1.0",
                    "targets": [
                        {"name": "crate-b", "kind": ["bin"], "src_path": "crates/b/src/main.rs"},
                        {"name": "demo", "kind": ["example"], "src_path": "crates/b/examples/demo.rs"}
                    ]
                }
            ],
            "workspace_members": ["crate-a 0.1.0", "crate-b 0.1.0"]
        }"#;
        let targets = parse_cargo_metadata(metadata, "workspace").unwrap();

        assert_eq!(targets.len(), 3);
        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"crate-a"));
        assert!(names.contains(&"crate-b"));
        assert!(names.contains(&"demo"));
    }

    #[test]
    fn test_cargo_target_type_cmp_order() {
        assert!(CargoTargetType::Binary.cmp_order() < CargoTargetType::Example.cmp_order());
    }
}
