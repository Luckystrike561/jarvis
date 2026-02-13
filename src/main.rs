//! # Jarvis CLI Entry Point
//!
//! This is the main entry point for the Jarvis TUI application.
//!
//! ## Overview
//!
//! Jarvis is a beautiful terminal user interface for discovering and executing
//! scripts with zero configuration. It automatically finds scripts in your project
//! and presents them in an organized, searchable interface.
//!
//! ## Usage
//!
//! ```bash
//! # Use current directory
//! jarvis
//!
//! # Use a specific directory
//! jarvis --path /path/to/project
//!
//! # Use a specific file
//! jarvis --file ./scripts/deploy.sh
//!
//! # Debug mode - print discovered scripts and exit
//! jarvis --debug
//! ```
//!
//! ## Architecture
//!
//! The application follows a simple architecture:
//!
//! 1. **Discovery**: Scans for script files in the project directory
//! 2. **Parsing**: Extracts functions/scripts from discovered files
//! 3. **UI**: Presents scripts in an interactive TUI with search and categories
//! 4. **Execution**: Runs selected scripts inline with PTY support
//!
//! ## Key Bindings
//!
//! ### Script List (left panel)
//! - `q` / `Q` - Quit the application
//! - `j` / `Down` - Move selection down
//! - `k` / `Up` - Move selection up
//! - `Enter` - Execute selected script or expand/collapse category
//! - `/` - Enter search mode
//! - `Tab` - Toggle focus between panes
//! - `i` - Show/hide info modal
//!
//! ### Output Panel (right panel)
//! - `j` / `k` - Scroll down/up by line
//! - `Ctrl+d` / `Ctrl+u` - Scroll down/up by half page
//! - `G` - Jump to bottom
//! - `gg` - Jump to top
//! - `v` - Enter visual selection mode
//! - `y` - Yank (copy) selected text to clipboard
//! - `Esc` / `q` - Return focus to left panel
//! - `Tab` - Switch pane

use jarvis::script;
use jarvis::ui;
use jarvis::ui::App;
use jarvis::usage::{UsageTracker, FREQUENTLY_USED_CATEGORY, MAX_FREQUENT_COMMANDS};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::panic;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Trait for reading terminal events (allows dependency injection for testing)
trait EventReader {
    fn read_event(&mut self, timeout: Duration) -> Result<Option<Event>>;
}

/// Production event reader that uses crossterm's event polling + read
struct CrosstermEventReader;

impl EventReader for CrosstermEventReader {
    fn read_event(&mut self, timeout: Duration) -> Result<Option<Event>> {
        if event::poll(timeout).context("Failed to poll for events")? {
            Ok(Some(
                event::read().context("Failed to read keyboard event")?,
            ))
        } else {
            Ok(None)
        }
    }
}

/// Jarvis - A beautiful TUI for managing and executing bash scripts
#[derive(Parser, Debug)]
#[command(name = "jarvis")]
#[command(author = "Luckystrike561")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Your trusted AI assistant for automating scripts", long_about = None)]
struct Args {
    /// Path to the base directory to search for bash scripts
    #[arg(short, long, value_name = "DIR", conflicts_with = "file")]
    path: Option<PathBuf>,

    /// Path to a single script file to run Jarvis on
    #[arg(
        short = 'f',
        long = "file",
        value_name = "FILE",
        conflicts_with = "path"
    )]
    file: Option<PathBuf>,

    /// Print debug information about discovered scripts and exit
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Set up panic hook to ensure terminal is restored on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Try to restore terminal state
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);

        // Call the original panic hook
        original_hook(panic_info);
    }));

    // Run the application and ensure cleanup happens
    let result = run_application(args).await;

    // Restore panic hook
    let _ = panic::take_hook();

    result
}

async fn run_application(args: Args) -> Result<()> {
    // Pre-warm tool availability checks in parallel (devbox, task, make, just, cargo, nx)
    // These run in background threads so they're ready by the time discovery needs them
    script::prewarm_tool_checks();

    // Determine script files based on mode: single file or directory discovery
    let (script_files, current_dir) = if let Some(file_path) = args.file {
        // Single file mode: discover only from the specified file
        let canonical_path = file_path
            .canonicalize()
            .with_context(|| format!("Failed to access file: {}", file_path.display()))?;

        let script_file = script::discover_single_file(&canonical_path)
            .with_context(|| format!("Failed to parse file: {}", canonical_path.display()))?;

        // Use the file's parent directory as the working directory
        let dir = canonical_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        (vec![script_file], dir)
    } else {
        // Directory mode: discover scripts from directory tree
        let current_dir = if let Some(path) = args.path {
            path.canonicalize()
                .with_context(|| format!("Failed to access directory: {}", path.display()))?
        } else {
            std::env::current_dir().context("Failed to get current working directory")?
        };

        // Discover scripts from multiple locations:
        // 1. Current directory (root .sh files only, depth 1 to avoid subdirs)
        // 2. ./script/ folder (if exists)
        // 3. ./scripts/ folder (if exists)
        // 4. ./jarvis/ folder (if exists)
        let mut script_files = Vec::new();

        // Scan current directory for .sh files (shallow, only immediate directory)
        let root_files = script::discover_scripts_shallow(&current_dir)
            .with_context(|| format!("Failed to discover scripts in: {}", current_dir.display()))?;
        script_files.extend(root_files);

        // Check optional subdirectories (with depth 2 for nested structures)
        let possible_dirs = vec!["script", "scripts", "jarvis"];
        for dir_name in possible_dirs {
            let dir_path = current_dir.join(dir_name);
            if dir_path.exists() && dir_path.is_dir() {
                let files = script::discover_scripts(&dir_path).with_context(|| {
                    format!("Failed to discover scripts in: {}", dir_path.display())
                })?;
                script_files.extend(files);
            }
        }

        if script_files.is_empty() {
            eprintln!("Warning: No scripts found");
            eprintln!("Searched in: {}", current_dir.display());
            eprintln!("Also checked: ./script/, ./scripts/, ./jarvis/ (if they exist)");
            eprintln!(
                "\nPlease add bash scripts (.sh), package.json, devbox.json, Taskfile.yml, Makefile, justfile, Cargo.toml, or nx.json to get started."
            );
            eprintln!("\nExample bash script format:");
            eprintln!(r#"  #!/usr/bin/env bash"#);
            eprintln!(r#"  my_function() {{"#);
            eprintln!(r#"      echo "Hello from my function""#);
            eprintln!(r#"  }}"#);
            eprintln!("\nExample package.json format:");
            eprintln!(r#"  {{"#);
            eprintln!(r#"    "scripts": {{"#);
            eprintln!(r#"      "start": "node index.js""#);
            eprintln!(r#"    }}"#);
            eprintln!(r#"  }}"#);
            std::process::exit(1);
        }

        (script_files, current_dir)
    };

    // Debug mode: print discovered scripts and exit
    if args.debug {
        println!("=== Discovered Script Files ===");
        for sf in &script_files {
            println!(
                "  Path: {}\n    Category: {}\n    Type: {:?}\n",
                sf.path.display(),
                sf.category,
                sf.script_type
            );
        }
        println!("\n=== Parsed Functions ===");
    }

    // Parse all scripts in parallel using threads for subprocess-heavy parsers
    enum ParseResult {
        Functions(Vec<script::ScriptFunction>),
        NxFunctions(
            Vec<script::ScriptFunction>,
            std::collections::HashMap<String, String>,
        ),
        Error(String, anyhow::Error),
    }

    let parse_handles: Vec<std::thread::JoinHandle<ParseResult>> = script_files
        .iter()
        .map(|script_file| {
            let path = script_file.path.clone();
            let category = script_file.category.clone();
            let script_type = script_file.script_type;

            std::thread::spawn(move || match script_type {
                script::ScriptType::Bash => match script::parse_script(&path, &category) {
                    Ok(functions) => {
                        let visible: Vec<_> =
                            functions.into_iter().filter(|f| !f.ignored).collect();
                        ParseResult::Functions(visible)
                    }
                    Err(e) => ParseResult::Error(path.display().to_string(), e),
                },
                script::ScriptType::PackageJson => {
                    match script::parse_package_json(&path, &category) {
                        Ok(npm_scripts) => {
                            let functions: Vec<script::ScriptFunction> = npm_scripts
                                .into_iter()
                                .map(|s| script::ScriptFunction {
                                    name: s.name,
                                    display_name: s.display_name,
                                    category: s.category,
                                    description: s.description,
                                    emoji: None,
                                    ignored: false,
                                    script_type: script::ScriptType::PackageJson,
                                })
                                .collect();
                            ParseResult::Functions(functions)
                        }
                        Err(e) => ParseResult::Error(path.display().to_string(), e),
                    }
                }
                script::ScriptType::DevboxJson => {
                    match script::parse_devbox_json(&path, &category) {
                        Ok(devbox_scripts) => {
                            let functions: Vec<script::ScriptFunction> = devbox_scripts
                                .into_iter()
                                .map(|s| script::ScriptFunction {
                                    name: s.name,
                                    display_name: s.display_name,
                                    category: s.category,
                                    description: s.description,
                                    emoji: None,
                                    ignored: false,
                                    script_type: script::ScriptType::DevboxJson,
                                })
                                .collect();
                            ParseResult::Functions(functions)
                        }
                        Err(e) => ParseResult::Error(path.display().to_string(), e),
                    }
                }
                script::ScriptType::Task => match script::list_tasks(&path, &category) {
                    Ok(tasks) => {
                        let functions: Vec<script::ScriptFunction> = tasks
                            .into_iter()
                            .filter(|t| !t.ignored)
                            .map(|t| script::ScriptFunction {
                                name: t.name,
                                display_name: t.display_name,
                                category: t.category,
                                description: t.description,
                                emoji: t.emoji,
                                ignored: t.ignored,
                                script_type: script::ScriptType::Task,
                            })
                            .collect();
                        ParseResult::Functions(functions)
                    }
                    Err(e) => ParseResult::Error(path.display().to_string(), e),
                },
                script::ScriptType::Makefile => match script::list_make_targets(&path, &category) {
                    Ok(targets) => {
                        let functions: Vec<script::ScriptFunction> = targets
                            .into_iter()
                            .filter(|t| !t.ignored)
                            .map(|t| script::ScriptFunction {
                                name: t.name,
                                display_name: t.display_name,
                                category: t.category,
                                description: t.description,
                                emoji: t.emoji,
                                ignored: t.ignored,
                                script_type: script::ScriptType::Makefile,
                            })
                            .collect();
                        ParseResult::Functions(functions)
                    }
                    Err(e) => ParseResult::Error(path.display().to_string(), e),
                },
                script::ScriptType::Just => match script::list_just_recipes(&path, &category) {
                    Ok(recipes) => {
                        let functions: Vec<script::ScriptFunction> = recipes
                            .into_iter()
                            .filter(|r| !r.ignored)
                            .map(|r| script::ScriptFunction {
                                name: r.name,
                                display_name: r.display_name,
                                category: r.category,
                                description: r.description,
                                emoji: r.emoji,
                                ignored: r.ignored,
                                script_type: script::ScriptType::Just,
                            })
                            .collect();
                        ParseResult::Functions(functions)
                    }
                    Err(e) => ParseResult::Error(path.display().to_string(), e),
                },
                script::ScriptType::CargoToml => {
                    match script::list_cargo_targets(&path, &category) {
                        Ok(targets) => {
                            let functions: Vec<script::ScriptFunction> = targets
                                .into_iter()
                                .filter(|t| !t.ignored)
                                .map(|t| {
                                    let prefixed_name = match t.target_type {
                                        script::cargo_parser::CargoTargetType::Binary => {
                                            format!("bin:{}", t.name)
                                        }
                                        script::cargo_parser::CargoTargetType::Example => {
                                            format!("example:{}", t.name)
                                        }
                                    };
                                    script::ScriptFunction {
                                        name: prefixed_name,
                                        display_name: t.display_name,
                                        category: t.category,
                                        description: t.description,
                                        emoji: t.emoji,
                                        ignored: t.ignored,
                                        script_type: script::ScriptType::CargoToml,
                                    }
                                })
                                .collect();
                            ParseResult::Functions(functions)
                        }
                        Err(e) => ParseResult::Error(path.display().to_string(), e),
                    }
                }
                script::ScriptType::NxJson => match script::list_nx_targets(&path, &category) {
                    Ok(nx_targets) => {
                        let display_names =
                            script::nx_parser::collect_category_display_names(&nx_targets);
                        let functions: Vec<script::ScriptFunction> = nx_targets
                            .into_iter()
                            .filter(|t| !t.ignored)
                            .map(|t| script::ScriptFunction {
                                name: t.name,
                                display_name: t.display_name,
                                category: t.category,
                                description: t.description,
                                emoji: t.emoji,
                                ignored: t.ignored,
                                script_type: script::ScriptType::NxJson,
                            })
                            .collect();
                        ParseResult::NxFunctions(functions, display_names)
                    }
                    Err(e) => ParseResult::Error(path.display().to_string(), e),
                },
            })
        })
        .collect();

    // Collect results from all threads
    let mut all_functions = Vec::new();
    let mut parse_errors = Vec::new();
    let mut nx_category_display_names = std::collections::HashMap::new();

    for handle in parse_handles {
        match handle.join() {
            Ok(ParseResult::Functions(functions)) => {
                all_functions.extend(functions);
            }
            Ok(ParseResult::NxFunctions(functions, display_names)) => {
                all_functions.extend(functions);
                nx_category_display_names.extend(display_names);
            }
            Ok(ParseResult::Error(path, err)) => {
                parse_errors.push((path, err));
            }
            Err(_) => {
                parse_errors.push((
                    "unknown".to_string(),
                    anyhow::anyhow!("Script parsing thread panicked"),
                ));
            }
        }
    }

    // Report any parse errors
    if !parse_errors.is_empty() {
        eprintln!("\nWarning: Failed to parse some scripts:");
        for (path, err) in &parse_errors {
            eprintln!("  - {}: {}", path, err);
        }
        eprintln!();
    }

    // Debug mode: print functions and exit
    if args.debug {
        for func in &all_functions {
            println!(
                "  Name: {}\n    Category: {}\n    Type: {:?}\n",
                func.name, func.category, func.script_type
            );
        }
        println!(
            "\nTotal: {} script files, {} functions",
            script_files.len(),
            all_functions.len()
        );
        return Ok(());
    }

    if all_functions.is_empty() {
        eprintln!("Error: No functions found in any scripts");
        eprintln!("\nMake sure your scripts define bash functions:");
        eprintln!(r#"  my_function() {{"#);
        eprintln!(r#"      echo "Hello""#);
        eprintln!(r#"  }}"#);
        eprintln!("\nAll bash functions are automatically discovered.");
        std::process::exit(1);
    }

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode for terminal")?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Create app with formatted project name
    let project_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project");

    let formatted_project_name = script::format_display_name(project_name);

    let mut app = App::new(all_functions.clone(), formatted_project_name);

    // Build category display names map from script files
    let mut category_display_names = std::collections::HashMap::new();
    for script_file in &script_files {
        category_display_names.insert(
            script_file.category.clone(),
            script_file.display_name.clone(),
        );
    }
    // Add per-project Nx category display names (one per Nx project)
    category_display_names.extend(nx_category_display_names);
    app.set_category_display_names(category_display_names);

    // Initialize usage tracking (gracefully handle errors)
    let usage_tracker = match UsageTracker::new(current_dir.clone()) {
        Ok(tracker) => Some(Arc::new(Mutex::new(tracker))),
        Err(e) => {
            eprintln!("Warning: Could not initialize usage tracking: {}", e);
            None
        }
    };

    // Load frequently used functions into the app
    if let Some(ref tracker) = usage_tracker {
        if let Ok(tracker_guard) = tracker.lock() {
            let frequent_entries = tracker_guard.get_frequent(MAX_FREQUENT_COMMANDS);
            let frequent_functions: Vec<script::ScriptFunction> = frequent_entries
                .iter()
                .filter_map(|entry| {
                    // Find the matching function in all_functions
                    all_functions
                        .iter()
                        .find(|f| {
                            f.name == entry.function_name && f.script_type == entry.script_type
                        })
                        .cloned()
                })
                .collect();
            app.set_frequent_functions(frequent_functions);
        }
    }

    // Run the app and ensure cleanup happens even on error
    let mut event_reader = CrosstermEventReader;
    let run_result = run_app(
        &mut terminal,
        &mut app,
        &script_files,
        &mut event_reader,
        usage_tracker.clone(),
    )
    .await;

    // Restore terminal (always runs, even if run_app failed)
    let cleanup_result = cleanup_terminal(&mut terminal);

    // Return the first error that occurred, or Ok if both succeeded
    run_result?;
    cleanup_result?;

    Ok(())
}

/// Clean up terminal state
fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to restore terminal")?;

    terminal.show_cursor().context("Failed to show cursor")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use std::collections::VecDeque;

    /// Mock event reader for testing that returns a predetermined sequence of events
    struct MockEventReader {
        events: VecDeque<Event>,
    }

    impl MockEventReader {
        fn new(events: Vec<Event>) -> Self {
            Self {
                events: VecDeque::from(events),
            }
        }
    }

    impl EventReader for MockEventReader {
        fn read_event(&mut self, _timeout: Duration) -> Result<Option<Event>> {
            Ok(self.events.pop_front())
        }
    }

    /// Helper to create a key event
    fn key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    #[test]
    fn test_mock_event_reader() {
        let events = vec![
            key_event(KeyCode::Char('a')),
            key_event(KeyCode::Char('b')),
            key_event(KeyCode::Enter),
        ];

        let mut reader = MockEventReader::new(events);

        // Should return events in order
        assert!(matches!(
            reader.read_event(Duration::from_millis(10)).unwrap(),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char('a'),
                ..
            }))
        ));
        assert!(matches!(
            reader.read_event(Duration::from_millis(10)).unwrap(),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Char('b'),
                ..
            }))
        ));
        assert!(matches!(
            reader.read_event(Duration::from_millis(10)).unwrap(),
            Some(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }))
        ));

        // Should return None when no more events
        assert!(reader
            .read_event(Duration::from_millis(10))
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_crossterm_event_reader_type() {
        // Just verify that CrosstermEventReader exists and implements the trait
        let _reader: Box<dyn EventReader> = Box::new(CrosstermEventReader);
    }

    #[tokio::test]
    async fn test_run_application_nonexistent_directory() {
        let args = Args {
            path: Some(PathBuf::from("/nonexistent/directory/that/does/not/exist")),
            file: None,
            debug: false,
        };

        let result = run_application(args).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to access directory"));
    }

    #[tokio::test]
    async fn test_run_application_file_instead_of_directory() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("notadir.txt");
        fs::write(&file_path, "test content").unwrap();

        let args = Args {
            path: Some(file_path.clone()),
            file: None,
            debug: false,
        };

        let result = run_application(args).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_args_parsing_with_path() {
        // Test that Args can parse path argument
        let args = Args {
            path: Some(PathBuf::from("/some/path")),
            file: None,
            debug: false,
        };
        assert_eq!(args.path, Some(PathBuf::from("/some/path")));
    }

    #[test]
    fn test_args_parsing_without_path() {
        // Test that Args works without path
        let args = Args {
            path: None,
            file: None,
            debug: false,
        };
        assert_eq!(args.path, None);
    }

    #[test]
    fn test_args_parsing_with_file() {
        // Test that Args can parse file argument
        let args = Args {
            path: None,
            file: Some(PathBuf::from("/some/file.sh")),
            debug: false,
        };
        assert_eq!(args.file, Some(PathBuf::from("/some/file.sh")));
    }

    #[tokio::test]
    async fn test_run_application_with_file_nonexistent() {
        let args = Args {
            path: None,
            file: Some(PathBuf::from("/nonexistent/file.sh")),
            debug: false,
        };

        let result = run_application(args).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to access file"));
    }

    #[tokio::test]
    async fn test_run_application_with_file_unsupported_type() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let txt_path = temp_dir.path().join("readme.txt");
        fs::write(&txt_path, "text content").unwrap();

        let args = Args {
            path: None,
            file: Some(txt_path),
            debug: false,
        };

        let result = run_application(args).await;
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        // The error chain includes "Failed to parse file" and "Unsupported file type"
        assert!(
            err_msg.contains("Unsupported file type") || err_msg.contains("Failed to parse file")
        );
    }
}

/// Execute a selected function inline using PTY
fn execute_inline(
    app: &mut App,
    func: &script::ScriptFunction,
    script_files: &[script::ScriptFile],
    usage_tracker: Option<Arc<Mutex<UsageTracker>>>,
    terminal_size: (u16, u16),
) -> Result<()> {
    let func_name = func.name.clone();

    // If the function is from "Frequently Used" category, find the original category
    let original_category = if func.category == FREQUENTLY_USED_CATEGORY {
        app.functions
            .iter()
            .find(|f| f.name == func_name && f.script_type == func.script_type)
            .map(|f| f.category.clone())
            .unwrap_or_else(|| func.category.clone())
    } else {
        func.category.clone()
    };

    // Find the script file
    if let Some(script_file) =
        ui::pty_runner::find_script_file(func, &original_category, script_files)
    {
        // If there's already a running PTY, finalize it first
        app.finalize_pty();

        // Calculate PTY size from the right panel area
        // The right panel is roughly 60% width, with 70% height for output
        // minus border characters
        let cols = (terminal_size.0 * 60 / 100).saturating_sub(2).max(40);
        let rows = (terminal_size.1 * 70 / 100).saturating_sub(4).max(10);

        // Spawn the command in a PTY
        let handle =
            ui::pty_runner::spawn_pty_command(func, script_file, &original_category, cols, rows)?;

        // Store the original function for tracking
        let mut tracking_func = func.clone();
        tracking_func.category = original_category.clone();
        app.active_function = Some(tracking_func);

        // Store the PTY handle
        app.pty_handle = Some(handle);

        // Reset output scroll to bottom (most recent)
        app.output_scroll = 0;

        // Focus on the output pane
        app.focus = ui::app::FocusPane::Output;

        // Store usage tracker reference for later (on completion)
        // We'll record usage when the PTY finishes successfully
        if let Some(tracker) = usage_tracker {
            // Store the tracker info so we can record on completion
            // We do this by spawning a thread that waits and records
            let func_name_clone = func_name.clone();
            let script_type = func.script_type;
            let category_clone = original_category;
            std::thread::spawn(move || {
                // Wait a bit then check periodically
                // The main loop handles finalization, so we don't need to do much here
                // Just record usage when the command finishes
                loop {
                    std::thread::sleep(Duration::from_millis(500));
                    if let Ok(tracker_guard) = tracker.lock() {
                        // We can't easily check if the PTY is done from here,
                        // so usage recording is handled in the main loop instead
                        drop(tracker_guard);
                        break;
                    }
                }
            });
            // Actually, let's handle this in the main loop instead
            // The thread above is not ideal. We'll track usage differently.
            let _ = func_name_clone;
            let _ = script_type;
            let _ = category_clone;
        }
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    script_files: &[script::ScriptFile],
    event_reader: &mut dyn EventReader,
    usage_tracker: Option<Arc<Mutex<UsageTracker>>>,
) -> Result<()> {
    // Track whether we need to record usage for completed commands
    let mut pending_usage_record: Option<(String, script::ScriptType, String)> = None;

    loop {
        // Check if a running PTY has completed
        if let Some(ref handle) = app.pty_handle {
            let status = handle.poll_status();
            if status == ui::pty_runner::ExecutionStatus::Succeeded
                || status == ui::pty_runner::ExecutionStatus::Failed
            {
                // Record the details before finalizing
                if status == ui::pty_runner::ExecutionStatus::Succeeded {
                    if let Some(ref func) = app.active_function {
                        pending_usage_record =
                            Some((func.name.clone(), func.script_type, func.category.clone()));
                    }
                }
                app.finalize_pty();
            }
        }

        // Process pending usage recording
        if let Some((func_name, script_type, category)) = pending_usage_record.take() {
            if let Some(ref tracker) = usage_tracker {
                if let Ok(mut tracker_guard) = tracker.lock() {
                    if let Err(e) = tracker_guard.record(&func_name, script_type, &category) {
                        eprintln!("Warning: Failed to record usage: {}", e);
                    }
                }
            }
        }

        terminal
            .draw(|f| ui::render(f, app))
            .context("Failed to draw terminal UI")?;

        // Use a short timeout for polling so we can update animations and PTY output
        let poll_timeout = if app.pty_handle.is_some() {
            Duration::from_millis(16) // ~60fps when a command is running
        } else {
            Duration::from_millis(100) // Normal or showing results
        };

        let event = event_reader.read_event(poll_timeout)?;

        // If no event, continue the loop (re-render for animations/PTY updates)
        let event = match event {
            Some(e) => e,
            None => continue,
        };

        if let Event::Key(key) = event {
            // Handle info modal close first
            if app.show_info {
                match key.code {
                    KeyCode::Char('i') | KeyCode::Esc => {
                        app.toggle_info();
                    }
                    _ => {}
                }
                continue;
            }

            // Handle search mode separately
            if app.search_mode {
                match key.code {
                    KeyCode::Esc => {
                        app.exit_search_mode();
                    }
                    KeyCode::Down => {
                        app.next();
                    }
                    KeyCode::Up => {
                        app.previous();
                    }
                    KeyCode::Backspace => {
                        app.search_pop_char();
                    }
                    KeyCode::Enter => {
                        // Execute function if one is selected
                        if let Some(ui::app::TreeItem::Function(func)) = app.selected_item() {
                            let size = terminal.size()?;
                            execute_inline(
                                app,
                                &func,
                                script_files,
                                usage_tracker.clone(),
                                (size.width, size.height),
                            )?;
                            app.exit_search_mode();
                        }
                    }
                    KeyCode::Char(c) => {
                        app.search_push_char(c);
                    }
                    _ => {}
                }
            } else if app.focus == ui::app::FocusPane::Output {
                // Output pane keybindings (Neovim-style)
                let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // Return focus to script list (don't quit)
                        app.focus = ui::app::FocusPane::ScriptList;
                        app.visual_mode = false;
                        app.selection_start = None;
                        app.selection_end = None;
                        app.pending_g = false;
                    }
                    KeyCode::Tab => {
                        app.toggle_focus();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.visual_mode {
                            // Move selection end down
                            if let Some(ref mut end) = app.selection_end {
                                end.0 += 1;
                            }
                        }
                        app.scroll_output_down();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.visual_mode {
                            // Move selection end up
                            if let Some(ref mut end) = app.selection_end {
                                end.0 = end.0.saturating_sub(1);
                            }
                        }
                        app.scroll_output_up();
                    }
                    KeyCode::Char('d') if has_ctrl => {
                        let size = terminal.size()?;
                        let visible = size.height.saturating_sub(8) as usize;
                        app.scroll_output_half_page_down(visible);
                    }
                    KeyCode::Char('u') if has_ctrl => {
                        let size = terminal.size()?;
                        let visible = size.height.saturating_sub(8) as usize;
                        app.scroll_output_half_page_up(visible);
                    }
                    KeyCode::Char('G') => {
                        // Jump to bottom
                        app.scroll_output_to_bottom();
                        app.pending_g = false;
                    }
                    KeyCode::Char('g') => {
                        if app.pending_g {
                            // gg: jump to top
                            app.scroll_output_to_top();
                            app.pending_g = false;
                        } else {
                            app.pending_g = true;
                        }
                    }
                    KeyCode::Char('v') => {
                        app.toggle_visual_mode();
                        app.pending_g = false;
                    }
                    KeyCode::Char('y') => {
                        app.yank_selection();
                        app.pending_g = false;
                    }
                    KeyCode::Char('i') => {
                        app.toggle_info();
                        app.pending_g = false;
                    }
                    _ => {
                        app.pending_g = false;
                    }
                }
            } else {
                // Normal mode keybindings (ScriptList or Details focus)
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('i') => {
                        app.toggle_info();
                    }
                    KeyCode::Char('/') => {
                        app.enter_search_mode();
                    }
                    KeyCode::Tab => {
                        app.toggle_focus();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.next();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.previous();
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if app.focus == ui::app::FocusPane::ScriptList {
                            app.handle_left();
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if app.focus == ui::app::FocusPane::ScriptList {
                            app.handle_right();
                        }
                    }
                    KeyCode::Enter => {
                        // Handle Enter based on selected item
                        if let Some(item) = app.selected_item() {
                            match item {
                                ui::app::TreeItem::Category(category) => {
                                    // Toggle category expansion
                                    app.toggle_category(&category);
                                }
                                ui::app::TreeItem::Function(func) => {
                                    let size = terminal.size()?;
                                    execute_inline(
                                        app,
                                        &func,
                                        script_files,
                                        usage_tracker.clone(),
                                        (size.width, size.height),
                                    )?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
