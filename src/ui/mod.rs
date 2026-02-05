//! # UI Module
//!
//! This module provides the terminal user interface components for Jarvis.
//!
//! ## Components
//!
//! - [`App`] - Application state management (selection, focus, search, etc.)
//! - [`mod@render`] - Rendering functions for drawing the TUI
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
//! │                     │      Output Panel         │
//! │                     │   (execution results)     │
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
//! - Scrollable output panel for execution results

pub mod app;
pub mod render;

pub use app::App;
pub use render::render;
