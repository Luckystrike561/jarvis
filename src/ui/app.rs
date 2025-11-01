use crate::script::ScriptFunction;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    MainMenu,
    CategoryView,
    Executing,
    ViewingOutput,
}

pub struct App {
    pub state: AppState,
    pub functions: Vec<ScriptFunction>,
    pub selected_index: usize,
    pub category_filter: Option<String>,
    pub output: Vec<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(functions: Vec<ScriptFunction>) -> Self {
        Self {
            state: AppState::MainMenu,
            functions,
            selected_index: 0,
            category_filter: None,
            output: Vec::new(),
            should_quit: false,
        }
    }
    
    pub fn next(&mut self) {
        let item_count = match self.state {
            AppState::MainMenu => self.categories().len(),
            _ => self.filtered_functions().len(),
        };
        
        if item_count > 0 {
            self.selected_index = (self.selected_index + 1) % item_count;
        }
    }
    
    pub fn previous(&mut self) {
        let item_count = match self.state {
            AppState::MainMenu => self.categories().len(),
            _ => self.filtered_functions().len(),
        };
        
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
        let items = self.filtered_functions();
        items.get(self.selected_index).copied()
    }
}
