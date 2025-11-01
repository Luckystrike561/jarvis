mod script;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Get the scripts and jarvis directories
    let current_dir = std::env::current_dir()?;
    let scripts_dir = current_dir.join("scripts");
    let jarvis_dir = current_dir.join("jarvis");

    // Check if at least one directory exists
    if !scripts_dir.exists() && !jarvis_dir.exists() {
        eprintln!("Error: Neither 'scripts' nor 'jarvis' directory found");
        eprintln!("Searched for:");
        eprintln!("  - {:?}", scripts_dir);
        eprintln!("  - {:?}", jarvis_dir);
        eprintln!("Please run from the jarvis directory");
        std::process::exit(1);
    }

    // Discover scripts from both directories
    let mut script_files = Vec::new();

    if scripts_dir.exists() {
        let mut files = script::discover_scripts(&scripts_dir)?;
        script_files.append(&mut files);
    }

    if jarvis_dir.exists() {
        let mut files = script::discover_scripts(&jarvis_dir)?;
        script_files.append(&mut files);
    }

    if script_files.is_empty() {
        eprintln!("Error: No bash scripts found in scripts or jarvis directories");
        std::process::exit(1);
    }

    // Parse all scripts
    let mut all_functions = Vec::new();
    for script_file in &script_files {
        let functions = script::parse_script(&script_file.path, &script_file.category)?;
        all_functions.extend(functions);
    }

    if all_functions.is_empty() {
        eprintln!("Warning: No functions found in scripts");
        eprintln!("Make sure your scripts have function arrays like:");
        eprintln!(r#"  fedora_functions=("Display Name:function_name" ...)"#);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(all_functions);

    // Run the app
    let res = run_app(&mut terminal, &mut app, &script_files).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

/// Suspend the TUI and restore terminal for interactive command execution
fn suspend_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Resume the TUI after interactive command execution
fn resume_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.hide_cursor()?;
    terminal.clear()?;
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    script_files: &[script::ScriptFile],
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
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
                        if let Some(item) = app.selected_item() {
                            if let ui::app::TreeItem::Function(func) = item {
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
                                    std::io::stdin().read_line(&mut input)?;

                                    // Store execution result in app output
                                    app.output.clear();
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
                        app.next();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.previous();
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.handle_left();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.handle_right();
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
                                        std::io::stdin().read_line(&mut input)?;

                                        // Store execution result in app output
                                        app.output.clear();
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
