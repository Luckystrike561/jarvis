//! # Application State Management
//!
//! This module contains the core application state and logic for the Jarvis TUI.
//!
//! ## Overview
//!
//! The [`App`] struct holds all application state including:
//! - List of discovered script functions
//! - Current selection and scroll positions
//! - Search mode and query state
//! - UI focus (which pane is active)
//! - Expanded/collapsed category state
//! - Frequently used commands tracking
//! - Inline terminal execution state with PTY
//! - Per-target command history (session-scoped)
//!
//! ## Navigation Model
//!
//! Scripts are displayed in a tree structure with categories:
//!
//! ```text
//! ▼ ⭐ Frequently Used    (pinned at top when usage exists)
//!   ├─ build
//!   └─ test
//! ▶ Category A          (collapsed)
//! ▼ Category B          (expanded)
//!   ├─ function_one
//!   └─ function_two
//! ▶ Category C          (collapsed)
//! ```
//!
//! The [`TreeItem`] enum represents items in this tree view.
//!
//! ## Focus Panes
//!
//! The UI has multiple focusable panes managed by [`FocusPane`]:
//! - `ScriptList` - The main script/category tree
//! - `Details` - The details panel showing script info
//! - `Output` - The output panel showing execution results (with inline terminal)

use crate::script::ScriptFunction;
use crate::ui::pty_runner::{CommandHistory, ExecutionStatus, PtyHandle};
use crate::ui::theme::Theme;
use crate::usage::FREQUENTLY_USED_CATEGORY;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPane {
    ScriptList,
    Details,
    Output,
}

pub struct App {
    pub functions: Vec<ScriptFunction>,
    /// Functions that appear in the "Frequently Used" category
    pub frequent_functions: Vec<ScriptFunction>,
    pub selected_index: usize,
    pub output_scroll: usize,
    pub script_scroll: usize,
    pub should_quit: bool,
    pub focus: FocusPane,
    pub expanded_categories: Vec<String>,
    pub search_mode: bool,
    pub search_query: String,
    pub show_info: bool,
    pub category_display_names: HashMap<String, String>,
    pub project_title: String,

    // --- Inline terminal execution state ---
    /// Currently running PTY process handle (if any)
    pub pty_handle: Option<PtyHandle>,
    /// Session-scoped command history keyed by target identifier
    pub command_history: CommandHistory,
    /// The function currently being executed or last executed
    pub active_function: Option<ScriptFunction>,
    /// Animation tick counter for running state border animation
    pub animation_tick: u64,
    /// Last animation update timestamp
    pub last_animation_tick: Instant,
    /// Whether the 'g' key was pressed (waiting for second 'g' for gg)
    pub pending_g: bool,

    // --- Mouse selection state (right pane only) ---
    /// Whether a mouse drag selection is in progress
    pub mouse_selecting: bool,
    /// Selection start position (row, col) in terminal output visible coordinates
    pub mouse_sel_start: Option<(usize, usize)>,
    /// Selection end position (row, col) in terminal output visible coordinates
    pub mouse_sel_end: Option<(usize, usize)>,
    /// The inner area of the output panel (set during render, used for mouse hit-testing)
    pub output_inner_area: Option<(u16, u16, u16, u16)>,

    // --- Theme state ---
    /// The active color theme
    pub theme: Theme,
    /// Whether the theme picker modal is visible
    pub show_theme_picker: bool,
    /// Currently highlighted index in the theme picker list
    pub theme_picker_index: usize,
}

impl App {
    pub fn new(functions: Vec<ScriptFunction>, project_title: String, theme: Theme) -> Self {
        // Find the theme's index in the built-in list (for the picker)
        let theme_picker_index = Theme::all()
            .iter()
            .position(|t| t.name == theme.name)
            .unwrap_or(0);

        Self {
            functions,
            frequent_functions: Vec::new(),
            selected_index: 0,
            output_scroll: 0,
            script_scroll: 0,
            should_quit: false,
            focus: FocusPane::ScriptList,
            expanded_categories: Vec::new(),
            search_mode: false,
            search_query: String::new(),
            show_info: false,
            category_display_names: HashMap::new(),
            project_title,
            pty_handle: None,
            command_history: CommandHistory::new(),
            active_function: None,
            animation_tick: 0,
            last_animation_tick: Instant::now(),
            pending_g: false,
            mouse_selecting: false,
            mouse_sel_start: None,
            mouse_sel_end: None,
            output_inner_area: None,
            theme,
            show_theme_picker: false,
            theme_picker_index,
        }
    }

    /// Set the frequently used functions to display in the special category
    pub fn set_frequent_functions(&mut self, functions: Vec<ScriptFunction>) {
        self.frequent_functions = functions;
    }

    pub fn set_category_display_names(&mut self, display_names: HashMap<String, String>) {
        self.category_display_names = display_names;
    }

    pub fn get_category_display_name(&self, category: &str) -> String {
        self.category_display_names
            .get(category)
            .cloned()
            .unwrap_or_else(|| category.to_string())
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::ScriptList => {
                if self.has_terminal_output() {
                    FocusPane::Output
                } else {
                    FocusPane::ScriptList
                }
            }
            FocusPane::Details => FocusPane::ScriptList,
            FocusPane::Output => FocusPane::ScriptList,
        };
        // Clear mouse selection when leaving output
        if self.focus != FocusPane::Output {
            self.clear_mouse_selection();
            self.pending_g = false;
        }
    }

    /// Check if there is terminal output to display for the currently selected function.
    /// Returns true if the selected function has a running PTY or completed history.
    pub fn has_terminal_output(&self) -> bool {
        let selected = self.selected_function();
        // Check if there's a running PTY for the selected function
        if self.pty_handle.is_some() {
            if let (Some(ref active), Some(ref sel)) = (&self.active_function, &selected) {
                if active.name == sel.name && active.script_type == sel.script_type {
                    return true;
                }
            }
        }
        // Check if the selected function has history
        if let Some(ref func) = selected {
            if self.command_history.get(func).is_some() {
                return true;
            }
        }
        false
    }

    /// Get the execution status for the currently selected function
    pub fn current_execution_status(&self) -> ExecutionStatus {
        let selected = self.selected_function();
        // If there's a running PTY and it belongs to the selected function, return its status
        if let Some(ref handle) = self.pty_handle {
            if let (Some(ref active), Some(ref sel)) = (&self.active_function, &selected) {
                if active.name == sel.name && active.script_type == sel.script_type {
                    return handle.poll_status();
                }
            }
        }
        // Check history for the selected function
        if let Some(ref func) = selected {
            if let Some(state) = self.command_history.get(func) {
                return state.status;
            }
        }
        ExecutionStatus::Idle
    }

    /// Update the animation tick (called from the event loop)
    pub fn tick_animation(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_animation_tick).as_millis() >= 100 {
            self.animation_tick = self.animation_tick.wrapping_add(1);
            self.last_animation_tick = now;
        }
    }

    /// Finalize a completed PTY handle: move it into command history
    pub fn finalize_pty(&mut self) {
        if let Some(handle) = self.pty_handle.take() {
            let status = handle.poll_status();
            if status == ExecutionStatus::Succeeded || status == ExecutionStatus::Failed {
                if let Some(ref func) = self.active_function {
                    let state = handle.into_execution_state();
                    self.command_history.insert(func, state);
                }
            } else {
                // Still running, put it back
                self.pty_handle = Some(handle);
            }
        }
    }

    /// Get the total number of scrollable lines in the terminal output for the selected function
    pub fn terminal_total_lines(&self) -> usize {
        let selected = self.selected_function();
        // Check running PTY if it belongs to the selected function
        if let Some(ref handle) = self.pty_handle {
            if let (Some(ref active), Some(ref sel)) = (&self.active_function, &selected) {
                if active.name == sel.name && active.script_type == sel.script_type {
                    return crate::ui::terminal_widget::total_content_lines(&handle.parser);
                }
            }
        }
        // Check history for the selected function
        if let Some(ref func) = selected {
            if let Some(state) = self.command_history.get(func) {
                return crate::ui::terminal_widget::total_content_lines(&state.parser);
            }
        }
        0
    }

    /// Scroll output down by half a page
    pub fn scroll_output_half_page_down(&mut self, visible_height: usize) {
        let half = visible_height / 2;
        let total = self.terminal_total_lines();
        let max_scroll = total.saturating_sub(visible_height);
        // output_scroll represents "how many lines from the bottom we've scrolled up"
        // So scrolling "down" (towards bottom) means decreasing the offset
        self.output_scroll = self.output_scroll.saturating_sub(half);
        let _ = max_scroll; // max_scroll not needed for down scroll
    }

    /// Scroll output up by half a page
    pub fn scroll_output_half_page_up(&mut self, visible_height: usize) {
        let half = visible_height / 2;
        let total = self.terminal_total_lines();
        let max_scroll = total.saturating_sub(visible_height);
        self.output_scroll = (self.output_scroll + half).min(max_scroll);
    }

    /// Jump to the bottom of output
    pub fn scroll_output_to_bottom(&mut self) {
        self.output_scroll = 0;
    }

    /// Jump to the top of output
    pub fn scroll_output_to_top(&mut self) {
        let total = self.terminal_total_lines();
        // Max scroll would show the very first line at the top
        self.output_scroll = total;
    }

    /// Clear any active mouse selection
    pub fn clear_mouse_selection(&mut self) {
        self.mouse_selecting = false;
        self.mouse_sel_start = None;
        self.mouse_sel_end = None;
    }

    /// Start a mouse drag selection at the given terminal-relative (row, col)
    pub fn start_mouse_selection(&mut self, row: usize, col: usize) {
        self.mouse_selecting = true;
        self.mouse_sel_start = Some((row, col));
        self.mouse_sel_end = Some((row, col));
    }

    /// Update the mouse drag selection end position
    pub fn update_mouse_selection(&mut self, row: usize, col: usize) {
        if self.mouse_selecting {
            self.mouse_sel_end = Some((row, col));
        }
    }

    /// Finish mouse selection and copy selected text to clipboard.
    /// Returns true if text was copied.
    pub fn finish_mouse_selection(&mut self) -> bool {
        if !self.mouse_selecting {
            return false;
        }
        self.mouse_selecting = false;

        let start = match self.mouse_sel_start {
            Some(s) => s,
            None => return false,
        };
        let end = match self.mouse_sel_end {
            Some(e) => e,
            None => return false,
        };

        // Don't copy if start == end (just a click, no drag)
        if start == end {
            self.clear_mouse_selection();
            return false;
        }

        let scroll_offset = self.output_scroll;
        let selected = self.selected_function();

        let text = if let Some(ref handle) = self.pty_handle {
            let is_selected =
                if let (Some(ref active), Some(ref sel)) = (&self.active_function, &selected) {
                    active.name == sel.name && active.script_type == sel.script_type
                } else {
                    false
                };
            if is_selected {
                crate::ui::terminal_widget::get_selected_text(
                    &handle.parser,
                    scroll_offset,
                    start,
                    end,
                )
            } else {
                String::new()
            }
        } else if let Some(ref func) = selected {
            if let Some(state) = self.command_history.get(func) {
                crate::ui::terminal_widget::get_selected_text(
                    &state.parser,
                    scroll_offset,
                    start,
                    end,
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !text.is_empty() {
            // Use OSC 52 escape sequence to set the system clipboard via the terminal.
            // Format: ESC ] 52 ; c ; <base64-encoded-text> BEL
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&text);
            let osc = format!("\x1b]52;c;{}\x07", encoded);
            let _ = std::io::Write::write_all(&mut std::io::stdout(), osc.as_bytes());
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }

        // Keep selection visible (don't clear highlight yet — cleared on next click)
        true
    }

    pub fn toggle_info(&mut self) {
        self.show_info = !self.show_info;
    }

    pub fn scroll_output_up(&mut self) {
        // Scrolling "up" means showing earlier content = increasing scroll offset
        let total = self.terminal_total_lines();
        let max_scroll = total;
        if self.output_scroll < max_scroll {
            self.output_scroll += 1;
        }
    }

    pub fn scroll_output_down(&mut self) {
        // Scrolling "down" means showing later content = decreasing scroll offset
        if self.output_scroll > 0 {
            self.output_scroll -= 1;
        }
    }

    pub fn reset_output_scroll(&mut self) {
        self.output_scroll = 0;
    }

    pub fn toggle_category(&mut self, category: &str) {
        if let Some(pos) = self.expanded_categories.iter().position(|c| c == category) {
            self.expanded_categories.remove(pos);
        } else {
            self.expanded_categories.push(category.to_string());
        }
    }

    pub fn is_category_expanded(&self, category: &str) -> bool {
        self.expanded_categories.contains(&category.to_string())
    }

    pub fn expand_category(&mut self, category: &str) {
        if !self.is_category_expanded(category) {
            self.expanded_categories.push(category.to_string());
        }
    }

    pub fn collapse_category(&mut self, category: &str) {
        if let Some(pos) = self.expanded_categories.iter().position(|c| c == category) {
            self.expanded_categories.remove(pos);
        }
    }

    // Handle left arrow: collapse category or move to parent category
    pub fn handle_left(&mut self) {
        if let Some(item) = self.selected_item() {
            match item {
                TreeItem::Category(category) => {
                    // Collapse if expanded
                    if self.is_category_expanded(&category) {
                        self.collapse_category(&category);
                    }
                }
                TreeItem::Function(func) => {
                    // Move to parent category
                    let items = self.tree_items();
                    // Find the category that contains this function
                    for (i, tree_item) in items.iter().enumerate() {
                        if let TreeItem::Category(cat) = tree_item {
                            if cat == &func.category && i < self.selected_index {
                                self.selected_index = i;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle right arrow: expand category or do nothing on function
    pub fn handle_right(&mut self) {
        if let Some(TreeItem::Category(category)) = self.selected_item() {
            // Expand if collapsed
            if !self.is_category_expanded(&category) {
                self.expand_category(&category);
            }
        }
    }

    // Get all items in tree view (categories and their functions)
    // The "Frequently Used" category appears first if there are any frequent functions
    pub fn tree_items(&self) -> Vec<TreeItem> {
        let mut items = Vec::new();

        // Add "Frequently Used" category first (if there are frequent functions)
        if !self.frequent_functions.is_empty() {
            let frequent_category = FREQUENTLY_USED_CATEGORY.to_string();

            // Filter frequent functions by search
            let frequent_funcs: Vec<&ScriptFunction> = self
                .frequent_functions
                .iter()
                .filter(|f| self.matches_search(f))
                .collect();

            // Only show if there are matching functions (when searching)
            if !self.search_mode || !frequent_funcs.is_empty() {
                items.push(TreeItem::Category(frequent_category.clone()));

                // Auto-expand when searching, or show if manually expanded
                if self.search_mode || self.is_category_expanded(&frequent_category) {
                    for func in frequent_funcs {
                        // Create a copy with the "Frequently Used" category
                        let mut freq_func = func.clone();
                        freq_func.category = frequent_category.clone();
                        items.push(TreeItem::Function(freq_func));
                    }
                }
            }
        }

        // Add regular categories
        let categories = self.regular_categories();

        for category in categories {
            // Filter functions for this category
            let funcs: Vec<&ScriptFunction> = self
                .functions
                .iter()
                .filter(|f| f.category == category)
                .filter(|f| self.matches_search(f))
                .collect();

            // Only show category if it has matching functions (when searching)
            if !self.search_mode || !funcs.is_empty() {
                items.push(TreeItem::Category(category.clone()));

                // Auto-expand categories when searching, or show if manually expanded
                if self.search_mode || self.is_category_expanded(&category) {
                    for func in funcs {
                        items.push(TreeItem::Function(func.clone()));
                    }
                }
            }
        }

        items
    }

    fn matches_search(&self, func: &ScriptFunction) -> bool {
        if !self.search_mode || self.search_query.is_empty() {
            return true;
        }

        let query = self.search_query.to_lowercase();
        func.display_name.to_lowercase().contains(&query)
            || func.name.to_lowercase().contains(&query)
            || func.description.to_lowercase().contains(&query)
            || func.category.to_lowercase().contains(&query)
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.selected_index = 0;
        self.reset_script_scroll();
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.selected_index = 0;
        self.reset_script_scroll();
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        self.selected_index = 0; // Reset selection when search changes
        self.reset_script_scroll();
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        self.selected_index = 0; // Reset selection when search changes
        self.reset_script_scroll();
    }

    pub fn selected_item(&self) -> Option<TreeItem> {
        let items = self.tree_items();
        items.get(self.selected_index).cloned()
    }

    /// Get the `ScriptFunction` for the currently selected tree item (if a function is selected).
    /// For "Frequently Used" entries, returns a copy with the original category
    /// so that `CommandHistory` lookups match the key used at execution time.
    pub fn selected_function(&self) -> Option<ScriptFunction> {
        if let Some(TreeItem::Function(func)) = self.selected_item() {
            if func.category == FREQUENTLY_USED_CATEGORY {
                // Find the original function to get the real category
                self.functions
                    .iter()
                    .find(|f| f.name == func.name && f.script_type == func.script_type)
                    .cloned()
            } else {
                Some(func)
            }
        } else {
            None
        }
    }

    pub fn next(&mut self) {
        let item_count = self.tree_items().len();

        if item_count > 0 {
            let old_index = self.selected_index;
            self.selected_index = (self.selected_index + 1) % item_count;
            if self.selected_index != old_index {
                self.output_scroll = 0;
                self.clear_mouse_selection();
            }
        }
    }

    pub fn previous(&mut self) {
        let item_count = self.tree_items().len();

        if item_count > 0 {
            let old_index = self.selected_index;
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = item_count - 1;
            }
            if self.selected_index != old_index {
                self.output_scroll = 0;
                self.clear_mouse_selection();
            }
        }
    }

    /// Ensure the selected item is visible within the scrolled viewport
    pub fn ensure_selected_visible(&mut self, visible_height: usize) {
        let item_count = self.tree_items().len();

        if item_count == 0 {
            return;
        }

        // If selected item is above the scroll window, scroll up
        if self.selected_index < self.script_scroll {
            self.script_scroll = self.selected_index;
        }

        // If selected item is below the scroll window, scroll down
        if self.selected_index >= self.script_scroll + visible_height {
            self.script_scroll = self.selected_index.saturating_sub(visible_height - 1);
        }
    }

    pub fn reset_script_scroll(&mut self) {
        self.script_scroll = 0;
    }

    /// Get all categories including the "Frequently Used" category if applicable
    pub fn categories(&self) -> Vec<String> {
        let mut cats = Vec::new();

        // Add "Frequently Used" first if there are frequent functions
        if !self.frequent_functions.is_empty() {
            cats.push(FREQUENTLY_USED_CATEGORY.to_string());
        }

        // Add regular categories
        cats.extend(self.regular_categories());

        cats
    }

    /// Get regular categories (excluding "Frequently Used")
    fn regular_categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .functions
            .iter()
            .map(|f| f.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    }
}

#[derive(Debug, Clone)]
pub enum TreeItem {
    Category(String),
    Function(ScriptFunction),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::ScriptType;

    fn test_theme() -> Theme {
        Theme::default_theme().clone()
    }

    fn create_test_functions() -> Vec<ScriptFunction> {
        vec![
            ScriptFunction {
                name: "func1".to_string(),
                display_name: "Function 1".to_string(),
                category: "System".to_string(),
                description: "Test function 1".to_string(),
                emoji: None,
                ignored: false,
                script_type: ScriptType::Bash,
            },
            ScriptFunction {
                name: "func2".to_string(),
                display_name: "Function 2".to_string(),
                category: "System".to_string(),
                description: "Test function 2".to_string(),
                emoji: None,
                ignored: false,
                script_type: ScriptType::Bash,
            },
            ScriptFunction {
                name: "func3".to_string(),
                display_name: "Function 3".to_string(),
                category: "Utilities".to_string(),
                description: "Test function 3".to_string(),
                emoji: None,
                ignored: false,
                script_type: ScriptType::Bash,
            },
        ]
    }

    #[test]
    fn test_app_new() {
        let functions = create_test_functions();
        let app = App::new(functions.clone(), "Test".to_string(), test_theme());

        assert_eq!(app.functions.len(), 3);
        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(!app.search_mode);
        assert_eq!(app.focus, FocusPane::ScriptList);
        assert_eq!(app.project_title, "Test");
        assert!(app.pty_handle.is_none());
        assert!(app.active_function.is_none());
        assert!(!app.mouse_selecting);
    }

    #[test]
    fn test_app_categories() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string(), test_theme());

        let categories = app.categories();
        assert_eq!(categories.len(), 2);
        assert!(categories.contains(&"System".to_string()));
        assert!(categories.contains(&"Utilities".to_string()));
    }

    #[test]
    fn test_app_toggle_category() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert!(!app.is_category_expanded("System"));

        app.toggle_category("System");
        assert!(app.is_category_expanded("System"));

        app.toggle_category("System");
        assert!(!app.is_category_expanded("System"));
    }

    #[test]
    fn test_app_expand_collapse_category() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        app.expand_category("System");
        assert!(app.is_category_expanded("System"));

        // Expanding again should not duplicate
        app.expand_category("System");
        assert_eq!(app.expanded_categories.len(), 1);

        app.collapse_category("System");
        assert!(!app.is_category_expanded("System"));

        // Collapsing again should be safe
        app.collapse_category("System");
        assert!(!app.is_category_expanded("System"));
    }

    #[test]
    fn test_app_navigation_next_previous() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert_eq!(app.selected_index, 0);

        app.next();
        assert_eq!(app.selected_index, 1);

        app.next();
        assert_eq!(app.selected_index, 0); // Wraps around

        app.previous();
        assert_eq!(app.selected_index, 1); // Goes to last

        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_app_toggle_focus() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert_eq!(app.focus, FocusPane::ScriptList);

        // No terminal output, so toggle stays on ScriptList
        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::ScriptList);
    }

    #[test]
    fn test_app_toggle_focus_with_output() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        // Expand a category and select a function
        app.expand_category("System");
        app.selected_index = 1; // First function under "System"

        // Add a command history entry for the selected function
        let func = app.selected_function().unwrap();
        let state = crate::ui::pty_runner::ExecutionState {
            status: ExecutionStatus::Succeeded,
            parser: std::sync::Arc::new(std::sync::Mutex::new(vt100::Parser::new(24, 80, 100))),
            exit_code: Some(0),
            started_at: Instant::now(),
            finished_at: Some(Instant::now()),
            display_name: func.display_name.clone(),
            category: func.category.clone(),
        };
        app.command_history.insert(&func, state);

        assert_eq!(app.focus, FocusPane::ScriptList);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::Output);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::ScriptList);
    }

    #[test]
    fn test_app_search_mode() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert!(!app.search_mode);
        assert_eq!(app.search_query, "");

        app.enter_search_mode();
        assert!(app.search_mode);
        assert_eq!(app.search_query, "");

        app.search_push_char('t');
        app.search_push_char('e');
        app.search_push_char('s');
        app.search_push_char('t');
        assert_eq!(app.search_query, "test");

        app.search_pop_char();
        assert_eq!(app.search_query, "tes");

        app.exit_search_mode();
        assert!(!app.search_mode);
        assert_eq!(app.search_query, "");
    }

    #[test]
    fn test_app_output_scroll() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert_eq!(app.output_scroll, 0);

        // Manually set scroll to test scroll_down behavior
        app.output_scroll = 3;

        app.scroll_output_down();
        assert_eq!(app.output_scroll, 2);

        app.scroll_output_down();
        assert_eq!(app.output_scroll, 1);

        app.scroll_output_down();
        assert_eq!(app.output_scroll, 0);

        // Should not go below 0
        app.scroll_output_down();
        assert_eq!(app.output_scroll, 0);

        app.reset_output_scroll();
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn test_app_toggle_info() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert!(!app.show_info);

        app.toggle_info();
        assert!(app.show_info);

        app.toggle_info();
        assert!(!app.show_info);
    }

    #[test]
    fn test_app_tree_items_collapsed() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string(), test_theme());

        let items = app.tree_items();
        // Should only show categories when collapsed
        assert_eq!(items.len(), 2); // System and Utilities

        match &items[0] {
            TreeItem::Category(name) => assert!(name == "System" || name == "Utilities"),
            _ => panic!("Expected category"),
        }
    }

    #[test]
    fn test_app_tree_items_expanded() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        app.expand_category("System");

        let items = app.tree_items();
        // Should show: System category + 2 functions + Utilities category
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_app_selected_item() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string(), test_theme());

        let item = app.selected_item();
        assert!(item.is_some());

        match item.unwrap() {
            TreeItem::Category(_) => {} // Expected
            _ => panic!("Expected category at index 0"),
        }
    }

    #[test]
    fn test_app_handle_left_right() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        // Initially not expanded
        assert!(!app.is_category_expanded("System"));

        // Simulate selecting first category (index 0)
        app.selected_index = 0;

        // Right arrow should expand
        app.handle_right();
        assert!(app.is_category_expanded("System"));

        // Left arrow should collapse
        app.handle_left();
        assert!(!app.is_category_expanded("System"));
    }

    #[test]
    fn test_app_empty_functions() {
        let app = App::new(vec![], "Test".to_string(), test_theme());

        assert_eq!(app.functions.len(), 0);
        assert_eq!(app.categories().len(), 0);
        assert_eq!(app.tree_items().len(), 0);
    }

    #[test]
    fn test_app_search_filtering() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        app.enter_search_mode();
        app.search_push_char('f');
        app.search_push_char('u');
        app.search_push_char('n');
        app.search_push_char('c');
        app.search_push_char('3');

        // Search should filter to only show items matching "func3"
        let items = app.tree_items();

        // Should show Utilities category + func3
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_app_frequent_functions() {
        let functions = create_test_functions();
        let mut app = App::new(functions.clone(), "Test".to_string(), test_theme());

        // No frequent functions initially
        assert!(app.frequent_functions.is_empty());
        assert_eq!(app.categories().len(), 2);

        // Set frequent functions
        app.set_frequent_functions(vec![functions[0].clone()]);

        // Should now have 3 categories (Frequently Used + System + Utilities)
        assert_eq!(app.categories().len(), 3);
        assert_eq!(app.categories()[0], FREQUENTLY_USED_CATEGORY);
    }

    #[test]
    fn test_app_tree_items_with_frequent() {
        let functions = create_test_functions();
        let mut app = App::new(functions.clone(), "Test".to_string(), test_theme());

        // Set frequent functions
        app.set_frequent_functions(vec![functions[0].clone()]);

        // Expand the frequently used category
        app.expand_category(FREQUENTLY_USED_CATEGORY);

        let items = app.tree_items();

        // Should show: Frequently Used category + 1 func + System category + Utilities category
        assert_eq!(items.len(), 4);

        // First item should be Frequently Used category
        match &items[0] {
            TreeItem::Category(name) => assert_eq!(name, FREQUENTLY_USED_CATEGORY),
            _ => panic!("Expected Frequently Used category first"),
        }

        // Second item should be the frequent function
        match &items[1] {
            TreeItem::Function(func) => {
                assert_eq!(func.name, "func1");
                assert_eq!(func.category, FREQUENTLY_USED_CATEGORY);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_app_frequent_search_filtering() {
        let functions = create_test_functions();
        let mut app = App::new(functions.clone(), "Test".to_string(), test_theme());

        // Set all functions as frequent
        app.set_frequent_functions(functions.clone());

        app.enter_search_mode();
        app.search_push_char('f');
        app.search_push_char('u');
        app.search_push_char('n');
        app.search_push_char('c');
        app.search_push_char('1');

        let items = app.tree_items();

        // Should show: Frequently Used + func1 + System + func1
        // (func1 appears in both Frequently Used and System)
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_app_execution_status_idle() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string(), test_theme());
        assert_eq!(app.current_execution_status(), ExecutionStatus::Idle);
    }

    #[test]
    fn test_app_mouse_selection() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        assert!(!app.mouse_selecting);

        app.start_mouse_selection(0, 0);
        assert!(app.mouse_selecting);
        assert_eq!(app.mouse_sel_start, Some((0, 0)));
        assert_eq!(app.mouse_sel_end, Some((0, 0)));

        app.update_mouse_selection(2, 5);
        assert_eq!(app.mouse_sel_end, Some((2, 5)));

        app.clear_mouse_selection();
        assert!(!app.mouse_selecting);
        assert!(app.mouse_sel_start.is_none());
        assert!(app.mouse_sel_end.is_none());
    }

    #[test]
    fn test_app_focus_clears_mouse_selection() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string(), test_theme());

        app.mouse_selecting = true;
        app.mouse_sel_start = Some((0, 0));
        app.mouse_sel_end = Some((1, 5));

        // Focus is ScriptList -> stays ScriptList (no terminal output)
        // but let's test that clearing works when focus != Output
        app.focus = FocusPane::Output;
        app.toggle_focus(); // Output -> ScriptList
        assert!(!app.mouse_selecting);
        assert!(app.mouse_sel_start.is_none());
    }
}
