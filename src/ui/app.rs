use crate::script::ScriptFunction;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    MainMenu,
    CategoryView,
    #[allow(dead_code)]
    Executing,
    #[allow(dead_code)]
    ViewingOutput,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPane {
    ScriptList,
    Details,
    Output,
}

pub struct App {
    pub state: AppState,
    pub functions: Vec<ScriptFunction>,
    pub selected_index: usize,
    pub category_filter: Option<String>,
    pub output: Vec<String>,
    pub output_scroll: usize,
    pub should_quit: bool,
    pub focus: FocusPane,
    pub expanded_categories: Vec<String>,
    pub search_mode: bool,
    pub search_query: String,
    pub show_info: bool,
}

impl App {
    pub fn new(functions: Vec<ScriptFunction>) -> Self {
        Self {
            state: AppState::MainMenu,
            functions,
            selected_index: 0,
            category_filter: None,
            output: Vec::new(),
            output_scroll: 0,
            should_quit: false,
            focus: FocusPane::ScriptList,
            expanded_categories: Vec::new(),
            search_mode: false,
            search_query: String::new(),
            show_info: false,
        }
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
        if let Some(item) = self.selected_item() {
            if let TreeItem::Category(category) = item {
                // Expand if collapsed
                if !self.is_category_expanded(&category) {
                    self.expand_category(&category);
                }
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
    }

    pub fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.selected_index = 0;
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        self.selected_index = 0; // Reset selection when search changes
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        self.selected_index = 0; // Reset selection when search changes
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

    pub fn filtered_functions(&self) -> Vec<&ScriptFunction> {
        match &self.category_filter {
            Some(category) => self
                .functions
                .iter()
                .filter(|f| &f.category == category)
                .collect(),
            None => self.functions.iter().collect(),
        }
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

    pub fn selected_function(&self) -> Option<&ScriptFunction> {
        if let Some(TreeItem::Function(func)) = self.selected_item() {
            // Find the function in our list by name
            self.functions.iter().find(|f| f.name == func.name)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum TreeItem {
    Category(String),
    Function(ScriptFunction),
}
