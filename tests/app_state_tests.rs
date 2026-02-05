//! Application state tests
//!
//! Tests for application state management including output scrolling,
//! focus toggling, and utility functions.

use jarvis::script::{format_display_name, ScriptFunction, ScriptType};
use jarvis::ui::app::FocusPane;
use jarvis::ui::App;

/// Helper to create a test app with mock functions
fn create_test_app() -> App {
    let functions = vec![
        ScriptFunction {
            name: "test_func1".to_string(),
            display_name: "Test Function 1".to_string(),
            category: "test_category".to_string(),
            description: "Test description 1".to_string(),
            emoji: Some("🚀".to_string()),
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
    App::new(functions, "Test Project".to_string())
}

#[tokio::test]
async fn test_output_scroll() {
    let mut app = create_test_app();
    app.focus = FocusPane::Output;

    // Add some output lines
    for i in 0..10 {
        app.output.push(format!("Line {}", i));
    }

    // Test scroll down
    app.scroll_output_down();

    // Test scroll up
    app.scroll_output_up();

    // Reset scroll
    app.reset_output_scroll();
}

#[tokio::test]
async fn test_focus_toggle() {
    let mut app = create_test_app();

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
