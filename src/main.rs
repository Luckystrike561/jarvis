mod script;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use script::format_display_name;
use std::io;
use std::panic;
use std::path::PathBuf;
use ui::App;

/// Jarvis - A beautiful TUI for managing and executing bash scripts
#[derive(Parser, Debug)]
#[command(name = "jarvis")]
#[command(author = "Luckystrike561")]
#[command(version = "0.1.0")]
#[command(about = "Your trusted AI assistant for automating scripts", long_about = None)]
struct Args {
    /// Path to the base directory to search for bash scripts
    #[arg(short, long, value_name = "DIR")]
    path: Option<PathBuf>,
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
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        
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
    // Get the base directory - use provided path or current directory
    let current_dir = if let Some(path) = args.path {
        path.canonicalize()
            .with_context(|| format!("Failed to access directory: {}", path.display()))?
    } else {
        std::env::current_dir()
            .context("Failed to get current working directory")?
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
            let files = script::discover_scripts(&dir_path)
                .with_context(|| format!("Failed to discover scripts in: {}", dir_path.display()))?;
            script_files.extend(files);
        }
    }

    if script_files.is_empty() {
        eprintln!("Warning: No bash scripts (.sh files) found");
        eprintln!("Searched in: {}", current_dir.display());
        eprintln!("Also checked: ./script/, ./scripts/, ./jarvis/ (if they exist)");
        eprintln!("\nPlease add bash scripts with functions to get started.");
        eprintln!("\nExample script format:");
        eprintln!(r#"  #!/usr/bin/env bash"#);
        eprintln!(r#"  my_function() {{"#);
        eprintln!(r#"      echo "Hello from my function""#);
        eprintln!(r#"  }}"#);
        std::process::exit(1);
    }

    // Parse all scripts
    let mut all_functions = Vec::new();
    let mut parse_errors = Vec::new();

    for script_file in &script_files {
        match script::parse_script(&script_file.path, &script_file.category) {
            Ok(functions) => {
                // Filter out ignored functions
                let visible_functions: Vec<_> = functions
                    .into_iter()
                    .filter(|f| !f.ignored)
                    .collect();
                all_functions.extend(visible_functions);
            }
            Err(e) => {
                parse_errors.push((script_file.path.display().to_string(), e));
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
    enable_raw_mode()
        .context("Failed to enable raw mode for terminal")?;
    
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to setup terminal")?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .context("Failed to create terminal")?;

    // Create app with formatted project name
    let project_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project");
    
    let formatted_project_name = format_display_name(project_name);
    
    let mut app = App::new(all_functions, formatted_project_name);
    
    // Build category display names map from script files
    let mut category_display_names = std::collections::HashMap::new();
    for script_file in &script_files {
        category_display_names.insert(
            script_file.category.clone(),
            script_file.display_name.clone(),
        );
    }
    app.set_category_display_names(category_display_names);

    // Run the app and ensure cleanup happens even on error
    let run_result = run_app(&mut terminal, &mut app, &script_files).await;

    // Restore terminal (always runs, even if run_app failed)
    let cleanup_result = cleanup_terminal(&mut terminal);

    // Return the first error that occurred, or Ok if both succeeded
    run_result?;
    cleanup_result?;

    Ok(())
}

/// Clean up terminal state
fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()
        .context("Failed to disable raw mode")?;
    
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to restore terminal")?;
    
    terminal.show_cursor()
        .context("Failed to show cursor")?;
    
    Ok(())
}

/// Suspend the TUI and restore terminal for interactive command execution
fn suspend_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()
        .context("Failed to disable raw mode when suspending TUI")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to leave alternate screen when suspending TUI")?;
    terminal.show_cursor()
        .context("Failed to show cursor when suspending TUI")?;
    Ok(())
}

/// Resume the TUI after interactive command execution
fn resume_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    enable_raw_mode()
        .context("Failed to enable raw mode when resuming TUI")?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )
    .context("Failed to enter alternate screen when resuming TUI")?;
    terminal.hide_cursor()
        .context("Failed to hide cursor when resuming TUI")?;
    terminal.clear()
        .context("Failed to clear terminal when resuming TUI")?;
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    script_files: &[script::ScriptFile],
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))
            .context("Failed to draw terminal UI")?;

        if let Event::Key(key) = event::read()
            .context("Failed to read keyboard event")? 
        {
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
                            // Execute function - clone data first
                            let func_name = func.name.clone();
                            let category = func.category.clone();
                            let display_name = func.display_name.clone();

                            // Find the script file
                            if let Some(script_file) =
                                script_files.iter().find(|s| s.category == category)
                            {
                                // Suspend TUI for interactive execution
                                suspend_tui(terminal)?;

                                // Clear screen and show execution message
                                println!("\n╔════════════════════════════════════════╗");
                                println!("║  Executing: {:<27}║", display_name);
                                println!("╚════════════════════════════════════════╝\n");

                                // Execute the function with full terminal access
                                let exit_code = script::execute_function_interactive(
                                    &script_file.path,
                                    &func_name,
                                )?;

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
                                app.output.push(format!("Category: {}", category));
                                app.output.push("".to_string());
                                if exit_code == 0 {
                                    app.output
                                        .push("Status: ✅ Completed successfully!".to_string());
                                } else {
                                    app.output.push(format!(
                                        "Status: ❌ Failed with exit code: {}",
                                        exit_code
                                    ));
                                }

                                // Resume TUI
                                resume_tui(terminal)?;
                            }
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
                                    // Execute function - clone data first
                                    let func_name = func.name.clone();
                                    let category = func.category.clone();
                                    let display_name = func.display_name.clone();

                                    // Find the script file
                                    if let Some(script_file) =
                                        script_files.iter().find(|s| s.category == category)
                                    {
                                        // Suspend TUI for interactive execution
                                        suspend_tui(terminal)?;

                                        // Clear screen and show execution message
                                        println!("\n╔════════════════════════════════════════╗");
                                        println!("║  Executing: {:<27}║", display_name);
                                        println!("╚════════════════════════════════════════╝\n");

                                        // Execute the function with full terminal access
                                        let exit_code = script::execute_function_interactive(
                                            &script_file.path,
                                            &func_name,
                                        )?;

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
                                    app.output.push(format!("Category: {}", category));
                                    app.output.push("".to_string());
                                    if exit_code == 0 {
                                        app.output
                                            .push("Status: ✅ Completed successfully!".to_string());
                                    } else {
                                        app.output.push(format!(
                                            "Status: ❌ Failed with exit code: {}",
                                            exit_code
                                        ));
                                    }

                                    // Resume TUI
                                        resume_tui(terminal)?;
                                    }
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
