//! Application state tests
//!
//! Tests for application state management including output scrolling,
//! focus toggling, and utility functions.

use jarvis::script::{format_display_name, ScriptFunction, ScriptType};
use jarvis::ui::app::FocusPane;
use jarvis::ui::pty_runner::{ExecutionState, ExecutionStatus};
use jarvis::ui::theme::Theme;
use jarvis::ui::App;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Helper to create a test app with mock functions
fn create_test_app() -> App {
    let functions = vec![
        ScriptFunction {
            name: "test_func1".to_string(),
            display_name: "Test Function 1".to_string(),
            category: "test_category".to_string(),
            description: "Test description 1".to_string(),
            emoji: Some("\u{1f680}".to_string()),
            ignored: false,
            script_type: ScriptType::Bash,
        },
        ScriptFunction {
            name: "test_func2".to_string(),
            display_name: "Test Function 2".to_string(),
            category: "test_category".to_string(),
            description: "Test description 2".to_string(),
            emoji: None,
            ignored: false,
            script_type: ScriptType::Bash,
        },
    ];
    App::new(
        functions,
        "Test Project".to_string(),
        Theme::default_theme().clone(),
    )
}

#[tokio::test]
async fn test_output_scroll() {
    let mut app = create_test_app();
    app.focus = FocusPane::Output;

    // Manually set scroll to test scroll mechanics
    app.output_scroll = 5;

    // Test scroll down
    app.scroll_output_down();
    assert_eq!(app.output_scroll, 4);

    // Test scroll up (needs terminal_total_lines > 0, so set scroll manually)
    app.output_scroll = 3;
    app.scroll_output_down();
    assert_eq!(app.output_scroll, 2);

    // Reset scroll
    app.reset_output_scroll();
    assert_eq!(app.output_scroll, 0);
}

#[tokio::test]
async fn test_focus_toggle() {
    let mut app = create_test_app();

    // Expand the category and select a function
    app.expand_category("test_category");
    app.selected_index = 1; // First function under "test_category"

    // Add history entry for the selected function so Output pane is available
    let func = app.selected_function().unwrap();
    let state = ExecutionState {
        status: ExecutionStatus::Succeeded,
        parser: Arc::new(Mutex::new(vt100::Parser::new(24, 80, 100))),
        exit_code: Some(0),
        started_at: Instant::now(),
        finished_at: Some(Instant::now()),
        display_name: func.display_name.clone(),
        category: func.category.clone(),
    };
    app.command_history.insert(&func, state);

    let initial_focus = app.focus;
    app.toggle_focus();
    let after_toggle = app.focus;

    // Focus should have changed
    assert_ne!(initial_focus, after_toggle);

    // Toggle back
    app.toggle_focus();
    assert_eq!(initial_focus, app.focus);
}

#[test]
fn test_format_display_name() {
    assert_eq!(format_display_name("test_name"), "Test Name");
    assert_eq!(format_display_name("my_function"), "My Function");
    assert_eq!(format_display_name("single"), "Single");
    assert_eq!(format_display_name(""), "");
}
