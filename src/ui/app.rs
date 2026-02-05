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
//!
//! ## Navigation Model
//!
//! Scripts are displayed in a tree structure with categories:
//!
//! ```text
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
//! - `Output` - The output panel showing execution results

use crate::script::ScriptFunction;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPane {
    ScriptList,
    Details,
    Output,
}

pub struct App {
    pub functions: Vec<ScriptFunction>,
    pub selected_index: usize,
    pub output: Vec<String>,
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
}

impl App {
    pub fn new(functions: Vec<ScriptFunction>, project_title: String) -> Self {
        Self {
            functions,
            selected_index: 0,
            output: Vec::new(),
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
        }
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
            FocusPane::ScriptList => FocusPane::Details,
            FocusPane::Details => {
                if !self.output.is_empty() {
                    FocusPane::Output
                } else {
                    FocusPane::ScriptList
                }
            }
            FocusPane::Output => FocusPane::ScriptList,
        };
    }

    pub fn toggle_info(&mut self) {
        self.show_info = !self.show_info;
    }

    pub fn scroll_output_up(&mut self) {
        if self.output_scroll > 0 {
            self.output_scroll -= 1;
        }
    }

    pub fn scroll_output_down(&mut self) {
        if self.output_scroll < self.output.len().saturating_sub(1) {
            self.output_scroll += 1;
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
    pub fn tree_items(&self) -> Vec<TreeItem> {
        let mut items = Vec::new();
        let categories = self.categories();

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

    pub fn next(&mut self) {
        let item_count = self.tree_items().len();

        if item_count > 0 {
            self.selected_index = (self.selected_index + 1) % item_count;
        }
    }

    pub fn previous(&mut self) {
        let item_count = self.tree_items().len();

        if item_count > 0 {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = item_count - 1;
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

    pub fn categories(&self) -> Vec<String> {
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
        let app = App::new(functions.clone(), "Test".to_string());

        assert_eq!(app.functions.len(), 3);
        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(!app.search_mode);
        assert_eq!(app.focus, FocusPane::ScriptList);
        assert_eq!(app.project_title, "Test");
    }

    #[test]
    fn test_app_categories() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string());

        let categories = app.categories();
        assert_eq!(categories.len(), 2);
        assert!(categories.contains(&"System".to_string()));
        assert!(categories.contains(&"Utilities".to_string()));
    }

    #[test]
    fn test_app_toggle_category() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string());

        assert!(!app.is_category_expanded("System"));

        app.toggle_category("System");
        assert!(app.is_category_expanded("System"));

        app.toggle_category("System");
        assert!(!app.is_category_expanded("System"));
    }

    #[test]
    fn test_app_expand_collapse_category() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string());

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
        let mut app = App::new(functions, "Test".to_string());

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
        let mut app = App::new(functions, "Test".to_string());

        assert_eq!(app.focus, FocusPane::ScriptList);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::Details);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::ScriptList);
    }

    #[test]
    fn test_app_toggle_focus_with_output() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string());

        // Add some output
        app.output.push("Test output".to_string());

        assert_eq!(app.focus, FocusPane::ScriptList);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::Details);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::Output);

        app.toggle_focus();
        assert_eq!(app.focus, FocusPane::ScriptList);
    }

    #[test]
    fn test_app_search_mode() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string());

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
        let mut app = App::new(functions, "Test".to_string());

        // Add multiple output lines
        for i in 0..10 {
            app.output.push(format!("Line {}", i));
        }

        assert_eq!(app.output_scroll, 0);

        app.scroll_output_down();
        assert_eq!(app.output_scroll, 1);

        app.scroll_output_down();
        assert_eq!(app.output_scroll, 2);

        app.scroll_output_up();
        assert_eq!(app.output_scroll, 1);

        app.scroll_output_up();
        assert_eq!(app.output_scroll, 0);

        // Should not go below 0
        app.scroll_output_up();
        assert_eq!(app.output_scroll, 0);

        app.reset_output_scroll();
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn test_app_toggle_info() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string());

        assert!(!app.show_info);

        app.toggle_info();
        assert!(app.show_info);

        app.toggle_info();
        assert!(!app.show_info);
    }

    #[test]
    fn test_app_tree_items_collapsed() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string());

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
        let mut app = App::new(functions, "Test".to_string());

        app.expand_category("System");

        let items = app.tree_items();
        // Should show: System category + 2 functions + Utilities category
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_app_selected_item() {
        let functions = create_test_functions();
        let app = App::new(functions, "Test".to_string());

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
        let mut app = App::new(functions, "Test".to_string());

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
        let app = App::new(vec![], "Test".to_string());

        assert_eq!(app.functions.len(), 0);
        assert_eq!(app.categories().len(), 0);
        assert_eq!(app.tree_items().len(), 0);
    }

    #[test]
    fn test_app_search_filtering() {
        let functions = create_test_functions();
        let mut app = App::new(functions, "Test".to_string());

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
}
