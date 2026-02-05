//! # Usage Tracking Module
//!
//! This module provides functionality for tracking and displaying the most
//! frequently used commands per project.
//!
//! ## Overview
//!
//! The usage tracking system helps users quickly access their most-used commands
//! by displaying a "Frequently Used" category at the top of the TUI.
//!
//! ## Storage
//!
//! Usage data is stored in XDG-compliant locations:
//! - Linux: `~/.local/share/jarvis/usage/`
//! - macOS: `~/Library/Application Support/jarvis/usage/`
//! - Windows: `%APPDATA%\jarvis\usage\`
//!
//! Each project has its own usage file, keyed by a hash of the project path.
//!
//! ## Data Format
//!
//! ```json
//! {
//!   "project_path": "/home/user/my-project",
//!   "entries": {
//!     "build": {
//!       "function_name": "build",
//!       "script_type": "Bash",
//!       "count": 42,
//!       "last_used": "2025-02-05T10:30:00Z"
//!     }
//!   }
//! }
//! ```

mod storage;

pub use storage::{
    ProjectUsage, UsageEntry, UsageTracker, FREQUENTLY_USED_CATEGORY, MAX_FREQUENT_COMMANDS,
};
