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
//!
//! ## Discovery
//!
//! Scripts are automatically discovered from:
//! - Root directory (`./`) - shallow scan
//! - `./script/` subdirectory
//! - `./scripts/` subdirectory
//! - `./jarvis/` subdirectory
//!
//! See [`discovery`] module for details.
//!
//! ## Execution
//!
//! All script types support interactive execution with full terminal access,
//! allowing scripts to use stdin/stdout/stderr directly.
//!
//! See [`executor`] module for details.

pub mod devbox_parser;
pub mod discovery;
pub mod executor;
pub mod npm_parser;
pub mod parser;
pub mod task_parser;

pub use devbox_parser::parse_devbox_json;
pub use discovery::{
    discover_scripts, discover_scripts_shallow, format_display_name, ScriptFile, ScriptType,
};
pub use executor::{
    execute_devbox_script_interactive, execute_function_interactive,
    execute_npm_script_interactive, execute_task_interactive,
};
pub use npm_parser::parse_package_json;
pub use parser::{parse_script, ScriptFunction};
pub use task_parser::list_tasks;
