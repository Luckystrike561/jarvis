//! # Script Module
//!
//! This module provides functionality for discovering, parsing, and executing
//! scripts from various sources.
//!
//! ## Supported Script Types
//!
//! | Type | File | Parser |
//! |------|------|--------|
//! | Bash | `*.sh` | [`parser::parse_script`] |
//! | npm | `package.json` | [`npm_parser::parse_package_json`] |
//! | Devbox | `devbox.json` | [`devbox_parser::parse_devbox_json`] |
//! | Task | `Taskfile.yml` | [`task_parser::list_tasks`] |
//! | Makefile | `Makefile` | [`makefile_parser::list_targets`] |
//! | Just | `justfile` | [`just_parser::list_recipes`] |
//! | Cargo | `Cargo.toml` | [`cargo_parser::list_targets`] |
//! | Nx | `nx.json` | [`nx_parser::list_targets`] |
//! | Terraform / `OpenTofu` | `*.tf` | [`terraform_parser::list_commands`] |
//! | Gradle | `build.gradle`, `build.gradle.kts` | [`gradle_parser::list_tasks`] |
//! | Bazel | `WORKSPACE`, `BUILD` | [`bazel_parser::list_targets`] |
//!

pub mod bazel_parser;
pub mod cargo_parser;
pub mod devbox_parser;
pub mod discovery;
pub mod gradle_parser;
pub mod just_parser;
pub mod makefile_parser;
pub mod npm_parser;
pub mod nx_parser;
pub mod parser;
pub mod task_parser;
pub mod terraform_parser;
pub mod utils;

pub use bazel_parser::list_targets as list_bazel_targets;
pub use cargo_parser::list_targets as list_cargo_targets;
pub use devbox_parser::parse_devbox_json;
pub use discovery::{
    discover_scripts, discover_scripts_shallow, discover_single_file, format_display_name,
    prewarm_tool_checks, ScriptFile, ScriptType,
};
pub use gradle_parser::list_tasks as list_gradle_tasks;
pub use just_parser::list_recipes as list_just_recipes;
pub use makefile_parser::list_targets as list_make_targets;
pub use npm_parser::parse_package_json;
pub use nx_parser::list_targets as list_nx_targets;
pub use parser::{parse_script, ScriptFunction};
pub use task_parser::list_tasks;
pub use terraform_parser::list_commands as list_terraform_commands;
