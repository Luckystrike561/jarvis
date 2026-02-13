//! # UI Rendering
//!
//! This module handles all rendering logic for the Jarvis TUI.
//!
//! ## Overview
//!
//! The [`render`] function is the main entry point that draws the entire UI
//! using the [ratatui] library. It composes multiple rendering helpers to
//! build the complete interface.
//!
//! ## Layout Structure
//!
//! The UI is rendered in layers:
//!
//! 1. **Header** - Project title and branding
//! 2. **Search Bar** - Visible when search mode is active
//! 3. **Body** - Split into left (script list) and right (details/output) panes
//! 4. **Footer** - Keyboard shortcuts help
//!
//! ## Rendering Helpers
//!
//! - `render_header` - Draws the top header bar
//! - `render_search_bar` - Draws the search input when active
//! - `render_script_tree` - Draws the categorized script list
//! - `render_details` - Draws the selected script details
//! - `render_terminal_output` - Draws inline terminal output from PTY
//! - `render_footer` - Draws the keyboard shortcuts
//! - `render_info_modal` - Draws the info popup overlay
//!
//! ## Border States
//!
//! The right panel border changes based on execution state:
//! - **Idle**: Default/dim gray border
//! - **Running**: Animated yellow/cyan border (spinning dots pattern)
//! - **Success**: Green border
//! - **Failure**: Red border

use crate::ui::app::{App, FocusPane, TreeItem};
use crate::ui::pty_runner::ExecutionStatus;
use crate::ui::terminal_widget::TerminalView;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Characters used for the spinning animation on the running border
const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub fn render(frame: &mut Frame, app: &mut App) {
    // Tick the animation
    app.tick_animation();

    // Main layout: Header + (optional Search) + Body + Footer
    let main_constraints = if app.search_mode {
        vec![
            Constraint::Length(3), // Header
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ]
    } else {
        vec![
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ]
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(main_constraints)
        .split(frame.area());

    let (body_idx, footer_idx) = if app.search_mode {
        // Render header
        render_header(frame, main_chunks[0]);
        // Render search bar
        render_search_bar(frame, app, main_chunks[1]);
        (2, 3)
    } else {
        // Render header
        render_header(frame, main_chunks[0]);
        (1, 2)
    };

    // Split body into left (scripts) and right (details/output)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[body_idx]);

    // Render script tree on left
    render_script_tree(frame, app, body_chunks[0]);

    // Split right side into details and output
    let has_terminal = app.has_terminal_output();
    if has_terminal {
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(body_chunks[1]);

        render_details(frame, app, right_chunks[0]);
        render_terminal_output(frame, app, right_chunks[1]);
    } else {
        render_details(frame, app, body_chunks[1]);
    }

    // Render footer
    render_footer(frame, app, main_chunks[footer_idx]);

    // Render info modal on top if show_info is true
    if app.show_info {
        render_info_modal(frame, frame.area());
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let header_text = vec![Line::from(vec![Span::styled(
        "  JARVIS - Just Another Rather Very Intelligent System  ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])];

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(header, area);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let search_text = format!("🔍 Search: {}", app.search_query);
    let search_widget = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Press ESC to exit search")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::Yellow));

    frame.render_widget(search_widget, area);
}

fn render_script_tree(frame: &mut Frame, app: &mut App, area: Rect) {
    let tree_items = app.tree_items();
    let total_items = tree_items.len();

    // Calculate visible viewport height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;

    // Ensure the selected item is visible
    app.ensure_selected_visible(visible_height);

    // Calculate the scrolled window
    let start_idx = app.script_scroll;
    let end_idx = (start_idx + visible_height).min(total_items);

    // Only render items within the visible window
    let items: Vec<ListItem> = tree_items[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(visible_i, item)| {
            let actual_i = start_idx + visible_i;
            let is_selected = actual_i == app.selected_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            match item {
                TreeItem::Category(category) => {
                    let is_expanded = app.is_category_expanded(category);
                    let icon = if is_expanded { "▼" } else { "▶" };
                    // Use the display name from the app (which includes emoji from filename)
                    let display_name = app.get_category_display_name(category);
                    let content = format!("{} {}", icon, display_name);
                    ListItem::new(content).style(style)
                }
                TreeItem::Function(func) => {
                    let emoji_prefix = func
                        .emoji
                        .as_ref()
                        .map(|e| format!("{} ", e))
                        .unwrap_or_default();
                    let content = format!("    {}{}", emoji_prefix, func.display_name);
                    ListItem::new(content).style(style)
                }
            }
        })
        .collect();

    let border_color = if app.focus == FocusPane::ScriptList {
        Color::Cyan
    } else {
        Color::Gray
    };

    // Create title with scroll position indicator if needed
    let title = if total_items > visible_height {
        format!(
            "🤖 {} [{}/{}]",
            app.project_title,
            start_idx + 1,
            total_items
        )
    } else {
        format!("🤖 {}", app.project_title)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(list, area);
}

fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus == FocusPane::Details {
        Color::Cyan
    } else {
        Color::Gray
    };

    let text = match app.selected_item() {
        Some(TreeItem::Category(category)) => {
            let count = app
                .functions
                .iter()
                .filter(|f| f.category == category)
                .count();

            let display_name = app.get_category_display_name(&category);

            vec![
                Line::from(vec![Span::styled(
                    display_name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(Color::Gray)),
                    Span::raw("Category"),
                ]),
                Line::from(vec![
                    Span::styled("Functions: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{}", count)),
                ]),
                Line::from(""),
                Line::from("────────────────────────────────────────"),
                Line::from(""),
                Line::from("Press Enter to expand/collapse"),
            ]
        }
        Some(TreeItem::Function(func)) => {
            vec![
                Line::from(vec![Span::styled(
                    func.display_name.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Gray)),
                    Span::raw(func.display_name.clone()),
                ]),
                Line::from(vec![
                    Span::styled("Function: ", Style::default().fg(Color::Gray)),
                    Span::raw(func.name.clone()),
                ]),
                Line::from(vec![
                    Span::styled("Category: ", Style::default().fg(Color::Gray)),
                    Span::raw(func.category.clone()),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Description:",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(func.description.clone()),
                Line::from(""),
                Line::from("────────────────────────────────────────"),
                Line::from(""),
                Line::from("Press Enter to execute this function"),
            ]
        }
        None => vec![
            Line::from("No item selected"),
            Line::from(""),
            Line::from("Use ↑↓ or j/k to navigate"),
        ],
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🖥️  Script Details")
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render the inline terminal output panel with PTY content
fn render_terminal_output(frame: &mut Frame, app: &App, area: Rect) {
    let status = app.current_execution_status();

    // Determine border color based on execution status
    let (border_color, border_modifier) = match status {
        ExecutionStatus::Idle => (
            if app.focus == FocusPane::Output {
                Color::Cyan
            } else {
                Color::Gray
            },
            Modifier::empty(),
        ),
        ExecutionStatus::Running => {
            // Animated border: alternate between colors
            let colors = [Color::Yellow, Color::Cyan, Color::Yellow, Color::White];
            let idx = (app.animation_tick as usize) % colors.len();
            (colors[idx], Modifier::BOLD)
        }
        ExecutionStatus::Succeeded => (Color::Green, Modifier::BOLD),
        ExecutionStatus::Failed => (Color::Red, Modifier::BOLD),
    };

    // Build title with status indicator
    let title = match status {
        ExecutionStatus::Idle => "💬 Output".to_string(),
        ExecutionStatus::Running => {
            let spinner = SPINNER_CHARS[(app.animation_tick as usize) % SPINNER_CHARS.len()];
            let name = app
                .pty_handle
                .as_ref()
                .map(|h| h.display_name.as_str())
                .unwrap_or("Command");
            format!("{} Running: {}", spinner, name)
        }
        ExecutionStatus::Succeeded => {
            let name = app
                .active_function
                .as_ref()
                .map(|f| f.display_name.as_str())
                .unwrap_or("Command");
            format!("✅ {}", name)
        }
        ExecutionStatus::Failed => {
            let name = app
                .active_function
                .as_ref()
                .map(|f| f.display_name.as_str())
                .unwrap_or("Command");
            let exit_code = if let Some(ref handle) = app.pty_handle {
                handle.poll_exit_code()
            } else if let Some(ref func) = app.active_function {
                app.command_history.get(func).and_then(|s| s.exit_code)
            } else {
                None
            };
            if let Some(code) = exit_code {
                format!("❌ {} (exit {})", name, code)
            } else {
                format!("❌ {}", name)
            }
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(
            Style::default()
                .fg(border_color)
                .add_modifier(border_modifier),
        );

    // Get the inner area (inside the border)
    let inner_area = block.inner(area);

    // Render the border block first
    frame.render_widget(block, area);

    // Now render the terminal content inside the border
    // Try to get the vt100 parser from active PTY or command history
    let parser_ref = if let Some(ref handle) = app.pty_handle {
        Some(&handle.parser)
    } else if let Some(ref func) = app.active_function {
        app.command_history.get(func).map(|s| &s.parser)
    } else {
        None
    };

    if let Some(parser) = parser_ref {
        let terminal_view = TerminalView::new(parser)
            .scroll_offset(app.output_scroll)
            .selection(app.visual_mode, app.selection_start, app.selection_end);
        frame.render_widget(terminal_view, inner_area);
    } else if !app.output.is_empty() {
        // Fallback: render legacy plain-text output
        let visible_height = inner_area.height as usize;
        let total_lines = app.output.len();
        let start_idx = app.output_scroll.min(total_lines.saturating_sub(1));
        let end_idx = (start_idx + visible_height).min(total_lines);

        let visible_output: Vec<Line> = app.output[start_idx..end_idx]
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        let paragraph = Paragraph::new(visible_output).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner_area);
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.search_mode {
        "[↑↓] Navigate  [Enter] Execute  [ESC] Exit Search  [Backspace] Delete"
    } else if app.output_search_mode {
        "[Enter] Confirm  [ESC] Cancel  [Backspace] Delete"
    } else {
        match app.focus {
            FocusPane::ScriptList => {
                "[↑↓/jk] Navigate  [←→/hl] Collapse/Expand  [/] Search  [i] Info  [Enter] Toggle/Execute  [Tab] Switch  [Q] Quit"
            }
            FocusPane::Details => "[Tab] Switch Pane  [/] Search  [i] Info  [Q] Quit",
            FocusPane::Output => {
                if app.visual_mode {
                    "[jk] Move selection  [y] Yank  [v/Esc] Exit visual  [Ctrl+d/u] Half-page"
                } else {
                    "[jk] Scroll  [Ctrl+d/u] Half-page  [G] Bottom  [gg] Top  [v] Visual  [Esc/q] Back  [Tab] Switch"
                }
            }
        }
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default());

    frame.render_widget(footer, area);
}

fn render_info_modal(frame: &mut Frame, area: Rect) {
    // Get version from Cargo.toml at compile time
    let version = env!("CARGO_PKG_VERSION");
    let authors = env!("CARGO_PKG_AUTHORS");
    let description = env!("CARGO_PKG_DESCRIPTION");

    // Create a centered modal
    let modal_width = 60;
    let modal_height = 14;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the background
    frame.render_widget(Clear, modal_area);

    // Create the modal content
    let info_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "JARVIS",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "Just Another Rather Very Intelligent System",
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from("────────────────────────────────────────────────────────"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(Color::Gray)),
            Span::styled(version, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Authors: ", Style::default().fg(Color::Gray)),
            Span::raw(authors),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            description,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press [i] or [ESC] to close",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let info_modal = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" About ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(info_modal, modal_area);
}
