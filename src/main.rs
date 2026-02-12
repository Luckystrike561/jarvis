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
//! 4. **Execution**: Runs selected scripts with full terminal access
//!
//! ## Key Bindings
//!
//! - `q` / `Q` - Quit the application
//! - `j` / `Down` - Move selection down
//! - `k` / `Up` - Move selection up
//! - `Enter` - Execute selected script or expand/collapse category
//! - `/` - Enter search mode
//! - `Tab` - Toggle focus between panes
//! - `i` - Show/hide info modal

use jarvis::script;
use jarvis::ui;
use jarvis::ui::App;
use jarvis::usage::{UsageTracker, FREQUENTLY_USED_CATEGORY, MAX_FREQUENT_COMMANDS};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::panic;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Trait for reading terminal events (allows dependency injection for testing)
trait EventReader {
    fn read_event(&mut self) -> Result<Event>;
}

/// Production event reader that uses crossterm's event::read()
struct CrosstermEventReader;

impl EventReader for CrosstermEventReader {
    fn read_event(&mut self) -> Result<Event> {
        event::read().context("Failed to read keyboard event")
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
                "\nPlease add bash scripts (.sh), package.json, devbox.json, Taskfile.yml, Makefile, justfile, or Cargo.toml to get started."
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

    // Parse all scripts
    let mut all_functions = Vec::new();
    let mut parse_errors = Vec::new();

    for script_file in &script_files {
        match &script_file.script_type {
            script::ScriptType::Bash => {
                match script::parse_script(&script_file.path, &script_file.category) {
                    Ok(functions) => {
                        // Filter out ignored functions
                        let visible_functions: Vec<_> =
                            functions.into_iter().filter(|f| !f.ignored).collect();
                        all_functions.extend(visible_functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            script::ScriptType::PackageJson => {
                match script::parse_package_json(&script_file.path, &script_file.category) {
                    Ok(npm_scripts) => {
                        // Convert NpmScript to ScriptFunction for TUI
                        let functions: Vec<script::ScriptFunction> = npm_scripts
                            .into_iter()
                            .map(|npm_script| script::ScriptFunction {
                                name: npm_script.name,
                                display_name: npm_script.display_name,
                                category: npm_script.category,
                                description: npm_script.description,
                                emoji: None, // npm scripts don't have emoji support yet
                                ignored: false, // npm scripts are never ignored
                                script_type: script::ScriptType::PackageJson,
                            })
                            .collect();
                        all_functions.extend(functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            script::ScriptType::DevboxJson => {
                match script::parse_devbox_json(&script_file.path, &script_file.category) {
                    Ok(devbox_scripts) => {
                        // Convert DevboxScript to ScriptFunction for TUI
                        let functions: Vec<script::ScriptFunction> = devbox_scripts
                            .into_iter()
                            .map(|devbox_script| script::ScriptFunction {
                                name: devbox_script.name,
                                display_name: devbox_script.display_name,
                                category: devbox_script.category,
                                description: devbox_script.description,
                                emoji: None, // devbox scripts don't have emoji support yet
                                ignored: false, // devbox scripts are never ignored
                                script_type: script::ScriptType::DevboxJson,
                            })
                            .collect();
                        all_functions.extend(functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            script::ScriptType::Task => {
                match script::list_tasks(&script_file.path, &script_file.category) {
                    Ok(task_tasks) => {
                        let functions: Vec<script::ScriptFunction> = task_tasks
                            .into_iter()
                            .filter(|t| !t.ignored) // Filter out ignored tasks
                            .map(|task_task| script::ScriptFunction {
                                name: task_task.name,
                                display_name: task_task.display_name,
                                category: task_task.category,
                                description: task_task.description,
                                emoji: task_task.emoji,
                                ignored: task_task.ignored,
                                script_type: script::ScriptType::Task,
                            })
                            .collect();
                        all_functions.extend(functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            script::ScriptType::Makefile => {
                match script::list_make_targets(&script_file.path, &script_file.category) {
                    Ok(make_targets) => {
                        let functions: Vec<script::ScriptFunction> = make_targets
                            .into_iter()
                            .filter(|t| !t.ignored) // Filter out ignored targets
                            .map(|make_target| script::ScriptFunction {
                                name: make_target.name,
                                display_name: make_target.display_name,
                                category: make_target.category,
                                description: make_target.description,
                                emoji: make_target.emoji,
                                ignored: make_target.ignored,
                                script_type: script::ScriptType::Makefile,
                            })
                            .collect();
                        all_functions.extend(functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            script::ScriptType::Just => {
                match script::list_just_recipes(&script_file.path, &script_file.category) {
                    Ok(just_recipes) => {
                        let functions: Vec<script::ScriptFunction> = just_recipes
                            .into_iter()
                            .filter(|r| !r.ignored) // Filter out ignored recipes
                            .map(|just_recipe| script::ScriptFunction {
                                name: just_recipe.name,
                                display_name: just_recipe.display_name,
                                category: just_recipe.category,
                                description: just_recipe.description,
                                emoji: just_recipe.emoji,
                                ignored: just_recipe.ignored,
                                script_type: script::ScriptType::Just,
                            })
                            .collect();
                        all_functions.extend(functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            script::ScriptType::CargoToml => {
                match script::list_cargo_targets(&script_file.path, &script_file.category) {
                    Ok(cargo_targets) => {
                        let functions: Vec<script::ScriptFunction> = cargo_targets
                            .into_iter()
                            .filter(|t| !t.ignored)
                            .map(|cargo_target| {
                                // Prefix name with target type for executor dispatch
                                let prefixed_name = match cargo_target.target_type {
                                    script::cargo_parser::CargoTargetType::Binary => {
                                        format!("bin:{}", cargo_target.name)
                                    }
                                    script::cargo_parser::CargoTargetType::Example => {
                                        format!("example:{}", cargo_target.name)
                                    }
                                };
                                script::ScriptFunction {
                                    name: prefixed_name,
                                    display_name: cargo_target.display_name,
                                    category: cargo_target.category,
                                    description: cargo_target.description,
                                    emoji: cargo_target.emoji,
                                    ignored: cargo_target.ignored,
                                    script_type: script::ScriptType::CargoToml,
                                }
                            })
                            .collect();
                        all_functions.extend(functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
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
    use crossterm::event::{KeyEvent, KeyModifiers};
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
        fn read_event(&mut self) -> Result<Event> {
            self.events
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("No more events in mock"))
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
            reader.read_event().unwrap(),
            Event::Key(KeyEvent {
                code: KeyCode::Char('a'),
                ..
            })
        ));
        assert!(matches!(
            reader.read_event().unwrap(),
            Event::Key(KeyEvent {
                code: KeyCode::Char('b'),
                ..
            })
        ));
        assert!(matches!(
            reader.read_event().unwrap(),
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            })
        ));

        // Should error when no more events
        assert!(reader.read_event().is_err());
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

/// Suspend the TUI and restore terminal for interactive command execution
fn suspend_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode when suspending TUI")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to leave alternate screen when suspending TUI")?;
    terminal
        .show_cursor()
        .context("Failed to show cursor when suspending TUI")?;
    Ok(())
}

/// Resume the TUI after interactive command execution
fn resume_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    enable_raw_mode().context("Failed to enable raw mode when resuming TUI")?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )
    .context("Failed to enter alternate screen when resuming TUI")?;
    terminal
        .hide_cursor()
        .context("Failed to hide cursor when resuming TUI")?;
    terminal
        .clear()
        .context("Failed to clear terminal when resuming TUI")?;
    Ok(())
}

/// Execute a selected function and handle the full execution flow
async fn execute_selected_function(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    func: &script::ScriptFunction,
    script_files: &[script::ScriptFile],
    usage_tracker: Option<Arc<Mutex<UsageTracker>>>,
) -> Result<()> {
    let func_name = func.name.clone();
    let display_name = func.display_name.clone();

    // If the function is from "Frequently Used" category, find the original category
    let original_category = if func.category == FREQUENTLY_USED_CATEGORY {
        // Look up the original function to get its real category
        app.functions
            .iter()
            .find(|f| f.name == func_name && f.script_type == func.script_type)
            .map(|f| f.category.clone())
            .unwrap_or_else(|| func.category.clone())
    } else {
        func.category.clone()
    };

    // Find the script file matching both category and script type
    if let Some(script_file) = script_files
        .iter()
        .find(|s| s.category == original_category && s.script_type == func.script_type)
    {
        // Suspend TUI for interactive execution
        suspend_tui(terminal)?;

        // Clear screen and show execution message
        println!("\n╔════════════════════════════════════════╗");
        println!("║  Executing: {:<27}║", display_name);
        println!("╚════════════════════════════════════════╝\n");

        // Execute based on script type
        let exit_code = match script_file.script_type {
            script::ScriptType::Bash => {
                script::execute_function_interactive(&script_file.path, &func_name)?
            }
            script::ScriptType::PackageJson => {
                // For npm scripts, pass the directory (parent of package.json)
                let package_dir = script_file.path.parent().with_context(|| {
                    format!(
                        "Failed to get parent directory of: {}",
                        script_file.path.display()
                    )
                })?;
                script::execute_npm_script_interactive(package_dir, &func_name)?
            }
            script::ScriptType::DevboxJson => {
                // For devbox scripts, pass the directory (parent of devbox.json)
                let devbox_dir = script_file.path.parent().with_context(|| {
                    format!(
                        "Failed to get parent directory of: {}",
                        script_file.path.display()
                    )
                })?;
                script::execute_devbox_script_interactive(devbox_dir, &func_name)?
            }
            script::ScriptType::Task => {
                script::execute_task_interactive(&script_file.path, &func_name)?
            }
            script::ScriptType::Makefile => {
                script::execute_make_target_interactive(&script_file.path, &func_name)?
            }
            script::ScriptType::Just => {
                script::execute_just_recipe_interactive(&script_file.path, &func_name)?
            }
            script::ScriptType::CargoToml => {
                script::execute_cargo_target_interactive(&script_file.path, &func_name)?
            }
        };

        // Show completion status
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        if exit_code == 0 {
            println!("✅ Completed successfully!");
        } else {
            println!("❌ Failed with exit code: {}", exit_code);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nPress Enter to return to JARVIS...");

        // Wait for user to press Enter
        let mut input = String::new();
        if let Err(e) = std::io::stdin().read_line(&mut input) {
            eprintln!("Warning: Failed to read input: {}", e);
        }

        // Store execution result in app output
        app.output.clear();
        app.reset_output_scroll();
        app.output.push(format!("Function: {}", display_name));
        app.output.push(format!("Category: {}", original_category));
        app.output.push("".to_string());
        if exit_code == 0 {
            app.output
                .push("Status: ✅ Completed successfully!".to_string());

            // Record usage on successful execution
            if let Some(ref tracker) = usage_tracker {
                if let Ok(mut tracker_guard) = tracker.lock() {
                    if let Err(e) =
                        tracker_guard.record(&func_name, func.script_type, &original_category)
                    {
                        eprintln!("Warning: Failed to record usage: {}", e);
                    }
                }
            }
        } else {
            app.output
                .push(format!("Status: ❌ Failed with exit code: {}", exit_code));
        }

        // Resume TUI
        resume_tui(terminal)?;
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
    loop {
        terminal
            .draw(|f| ui::render(f, app))
            .context("Failed to draw terminal UI")?;

        if let Event::Key(key) = event_reader.read_event()? {
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
                            execute_selected_function(
                                terminal,
                                app,
                                &func,
                                script_files,
                                usage_tracker.clone(),
                            )
                            .await?;
                        }
                    }
                    KeyCode::Char(c) => {
                        app.search_push_char(c);
                    }
                    _ => {}
                }
            } else {
                // Normal mode keybindings
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
                        if app.focus == ui::app::FocusPane::Output {
                            app.scroll_output_down();
                        } else {
                            app.next();
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.focus == ui::app::FocusPane::Output {
                            app.scroll_output_up();
                        } else {
                            app.previous();
                        }
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
                                    execute_selected_function(
                                        terminal,
                                        app,
                                        &func,
                                        script_files,
                                        usage_tracker.clone(),
                                    )
                                    .await?;
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
