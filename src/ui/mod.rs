//! # UI Module
//!
//! This module provides the terminal user interface components for Jarvis.
//!
//! ## Components
//!
//! - [`App`] - Application state management (selection, focus, search, etc.)
//! - [`mod@render`] - Rendering functions for drawing the TUI
//! - [`mod@pty_runner`] - PTY-based command execution for inline terminal
//! - [`mod@terminal_widget`] - Embedded terminal widget for rendering PTY output
//!
//! ## Layout
//!
//! The UI is organized into several panes:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                    Header                        │
//! ├─────────────────────┬───────────────────────────┤
//! │                     │                           │
//! │   Script List       │      Details Panel        │
//! │   (categories &     │   (description, emoji)    │
//! │    functions)       │                           │
//! │                     ├───────────────────────────┤
//! │                     │    Terminal Output         │
//! │                     │   (inline PTY execution)  │
//! │                     │                           │
//! ├─────────────────────┴───────────────────────────┤
//! │                    Footer                        │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - Tree-based navigation with collapsible categories
//! - Fuzzy search across all scripts
//! - Focus switching between panes with Tab
//! - Inline terminal execution with full PTY support
//! - Animated/colored borders showing execution status
//! - Neovim-style keybinds for output navigation
//! - Visual selection mode with clipboard copy

pub mod app;
pub mod pty_runner;
pub mod render;
pub mod terminal_widget;

pub use app::App;
pub use render::render;
