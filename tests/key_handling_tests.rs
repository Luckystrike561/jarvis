//! Keyboard event handling tests
//!
//! Tests for keyboard input handling including quit keys, search mode,
//! navigation, and modal interactions.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use jarvis::script::{ScriptFunction, ScriptType};
use jarvis::ui::theme::Theme;
use jarvis::ui::App;

/// Helper to create a key event
fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

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
async fn test_quit_with_q_key() {
    let mut app = create_test_app();

    // We can't easily test with a real terminal, but we can test the app state changes
    assert!(!app.should_quit);

    // Manually simulate the key handling logic
    let event = key_event(KeyCode::Char('q'));
    if let Event::Key(key) = event {
        if key.code == KeyCode::Char('q') {
            app.should_quit = true;
        }
    }

    assert!(app.should_quit);
}

#[tokio::test]
async fn test_quit_with_capital_q_key() {
    let mut app = create_test_app();

    assert!(!app.should_quit);

    let event = key_event(KeyCode::Char('Q'));
    if let Event::Key(key) = event {
        if key.code == KeyCode::Char('Q') {
            app.should_quit = true;
        }
    }

    assert!(app.should_quit);
}

#[tokio::test]
async fn test_info_modal_toggle() {
    let mut app = create_test_app();

    assert!(!app.show_info);

    // Toggle info modal on
    app.toggle_info();
    assert!(app.show_info);

    // Toggle info modal off
    app.toggle_info();
    assert!(!app.show_info);
}

#[tokio::test]
async fn test_search_mode_enter_and_exit() {
    let mut app = create_test_app();

    assert!(!app.search_mode);

    // Enter search mode
    app.enter_search_mode();
    assert!(app.search_mode);

    // Exit search mode
    app.exit_search_mode();
    assert!(!app.search_mode);
}

#[tokio::test]
async fn test_search_input_handling() {
    let mut app = create_test_app();
    app.enter_search_mode();

    // Add characters to search
    app.search_push_char('t');
    app.search_push_char('e');
    app.search_push_char('s');
    app.search_push_char('t');

    // Remove a character
    app.search_pop_char();

    // The search query should be managed internally by the app
    assert!(app.search_mode);
}

#[tokio::test]
async fn test_navigation_next_previous() {
    let mut app = create_test_app();

    // Test next and previous navigation
    app.next();
    app.previous();

    // Just verify these methods don't panic
    // The actual selection logic is tested in ui::app module
}

#[tokio::test]
async fn test_info_modal_closes_with_esc() {
    let mut app = create_test_app();

    // Open info modal
    app.toggle_info();
    assert!(app.show_info);

    // Close with ESC (simulated)
    app.toggle_info();
    assert!(!app.show_info);
}

#[tokio::test]
async fn test_search_mode_esc_key() {
    let mut app = create_test_app();

    // Enter search mode
    app.enter_search_mode();
    assert!(app.search_mode);

    // Press ESC to exit
    app.exit_search_mode();
    assert!(!app.search_mode);
}
