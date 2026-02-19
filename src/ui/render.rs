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
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
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
        render_header(frame, app, main_chunks[0]);
        // Render search bar
        render_search_bar(frame, app, main_chunks[1]);
        (2, 3)
    } else {
        // Render header
        render_header(frame, app, main_chunks[0]);
        (1, 2)
    };

    // Split body into left (scripts) and right (details/output)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(main_chunks[body_idx]);

    // Render script tree on left
    render_script_tree(frame, app, body_chunks[0]);

    // Render right side: terminal output (or empty state)
    let has_terminal = app.has_terminal_output();
    if has_terminal {
        render_terminal_output(frame, app, body_chunks[1]);
    } else {
        render_empty_output(frame, app, body_chunks[1]);
    }

    // Render footer
    render_footer(frame, app, main_chunks[footer_idx]);

    // Render info modal on top if show_info is true
    if app.show_info {
        let full_area = frame.area();
        render_info_modal(frame, app, full_area);
    }

    // Render theme picker modal on top if show_theme_picker is true
    if app.show_theme_picker {
        let full_area = frame.area();
        render_theme_picker(frame, app, full_area);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    // Build header: JARVIS branding on the left, selected item details on the right
    let mut spans = vec![Span::styled(
        "  JARVIS  ",
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    )];

    // Append selected item details inline
    match app.selected_item() {
        Some(TreeItem::Function(func)) => {
            spans.push(Span::styled(
                "\u{2502} ",
                Style::default().fg(app.theme.fg_dim),
            ));
            spans.push(Span::styled(
                func.display_name.clone(),
                Style::default()
                    .fg(app.theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ));
            if !func.description.is_empty() {
                spans.push(Span::styled("  ", Style::default()));
                spans.push(Span::styled(
                    func.description.clone(),
                    Style::default()
                        .fg(app.theme.fg)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }
        Some(TreeItem::Category(category)) => {
            let display_name = app.get_category_display_name(&category);
            let count = app
                .functions
                .iter()
                .filter(|f| f.category == category)
                .count();
            spans.push(Span::styled(
                "\u{2502} ",
                Style::default().fg(app.theme.fg_dim),
            ));
            spans.push(Span::styled(
                display_name,
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!("  ({} functions)", count),
                Style::default().fg(app.theme.fg_dim),
            ));
        }
        None => {}
    }

    let header_text = vec![Line::from(spans)];

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .style(Style::default().bg(app.theme.bg));

    frame.render_widget(header, area);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let search_text = format!("\u{1f50d} Search: {}", app.search_query);
    let search_widget = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Press ESC to exit search")
                .border_style(Style::default().fg(app.theme.secondary)),
        )
        .style(Style::default().fg(app.theme.secondary));

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
                    .fg(app.theme.bg)
                    .bg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.fg)
            };

            match item {
                TreeItem::Category(category) => {
                    let is_expanded = app.is_category_expanded(category);
                    let icon = if is_expanded { "\u{25bc}" } else { "\u{25b6}" };
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
        app.theme.accent
    } else {
        app.theme.fg_dim
    };

    // Create title with scroll position indicator if needed
    let title = if total_items > visible_height {
        format!(
            "\u{1f916} {} [{}/{}]",
            app.project_title,
            start_idx + 1,
            total_items
        )
    } else {
        format!("\u{1f916} {}", app.project_title)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(app.theme.fg));

    frame.render_widget(list, area);
}

/// Render a placeholder when no command has been run yet
fn render_empty_output(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.focus == FocusPane::Output {
        app.theme.accent
    } else {
        app.theme.fg_dim
    };

    let text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Select a function and press Enter to run it",
            Style::default().fg(app.theme.fg_dim),
        )]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("\u{1f4ac} Output")
                .border_style(Style::default().fg(border_color)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render the inline terminal output panel with PTY content
fn render_terminal_output(frame: &mut Frame, app: &mut App, area: Rect) {
    let status = app.current_execution_status();
    let selected = app.selected_function();

    // Determine if the running PTY belongs to the selected function
    let pty_is_selected = if let (Some(ref handle), Some(ref active), Some(ref sel)) =
        (&app.pty_handle, &app.active_function, &selected)
    {
        let _ = handle; // used below for parser
        active.name == sel.name && active.script_type == sel.script_type
    } else {
        false
    };

    // Determine border color based on execution status
    let (border_color, border_modifier) = match status {
        ExecutionStatus::Idle => (
            if app.focus == FocusPane::Output {
                app.theme.accent
            } else {
                app.theme.fg_dim
            },
            Modifier::empty(),
        ),
        ExecutionStatus::Running => {
            // Steady yellow border while running (spinner in title provides animation)
            (app.theme.secondary, Modifier::BOLD)
        }
        ExecutionStatus::Succeeded => (app.theme.success, Modifier::BOLD),
        ExecutionStatus::Failed => (app.theme.error, Modifier::BOLD),
    };

    // Build title with status indicator
    let display_name = selected
        .as_ref()
        .map(|f| f.display_name.as_str())
        .unwrap_or("Command");

    let title = match status {
        ExecutionStatus::Idle => "\u{1f4ac} Output".to_string(),
        ExecutionStatus::Running => {
            let spinner = SPINNER_CHARS[(app.animation_tick as usize) % SPINNER_CHARS.len()];
            format!("{} Running: {}", spinner, display_name)
        }
        ExecutionStatus::Succeeded => {
            format!("✅ {}", display_name)
        }
        ExecutionStatus::Failed => {
            let exit_code = if pty_is_selected {
                app.pty_handle
                    .as_ref()
                    .and_then(super::pty_runner::PtyHandle::poll_exit_code)
            } else if let Some(ref func) = selected {
                app.command_history.get(func).and_then(|s| s.exit_code)
            } else {
                None
            };
            if let Some(code) = exit_code {
                format!("❌ {} (exit {})", display_name, code)
            } else {
                format!("❌ {}", display_name)
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
    // Resolve the vt100 parser: running PTY (if selected) or history for the selected function
    let parser_ref = if pty_is_selected {
        app.pty_handle.as_ref().map(|h| &h.parser)
    } else if let Some(ref func) = selected {
        app.command_history.get(func).map(|s| &s.parser)
    } else {
        None
    };

    if let Some(parser) = parser_ref {
        // Use mouse selection state for highlight
        let has_selection = app.mouse_sel_start.is_some() && app.mouse_sel_end.is_some();
        let terminal_view = TerminalView::new(parser)
            .scroll_offset(app.output_scroll)
            .selection(has_selection, app.mouse_sel_start, app.mouse_sel_end)
            .selection_bg(app.theme.selection_bg);
        frame.render_widget(terminal_view, inner_area);
    }

    // Store inner area for mouse hit-testing in the event loop
    app.output_inner_area = Some((
        inner_area.x,
        inner_area.y,
        inner_area.width,
        inner_area.height,
    ));
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = if app.search_mode {
        "[\u{2191}\u{2193}] Navigate  [Enter] Execute  [ESC] Exit Search  [Backspace] Delete"
    } else {
        match app.focus {
            FocusPane::Details | FocusPane::ScriptList => {
                "[\u{2191}\u{2193}/jk] Navigate  [\u{2190}\u{2192}/hl] Collapse/Expand  [/] Search  [t] Theme  [i] Info  [Enter] Toggle/Execute  [Tab] Switch  [Q] Quit"
            }
            FocusPane::Output => {
                "[jk] Scroll  [Ctrl+d/u] Half-page  [G] Bottom  [gg] Top  [Mouse] Select+Copy  [Esc/q] Back  [Tab] Switch"
            }
        }
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(app.theme.fg_dim))
        .block(Block::default());

    frame.render_widget(footer, area);
}

fn render_info_modal(frame: &mut Frame, app: &App, area: Rect) {
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
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            "Just Another Rather Very Intelligent System",
            Style::default().fg(app.theme.fg),
        )]),
        Line::from(""),
        Line::from("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(app.theme.fg_dim)),
            Span::styled(version, Style::default().fg(app.theme.secondary)),
        ]),
        Line::from(vec![
            Span::styled("Authors: ", Style::default().fg(app.theme.fg_dim)),
            Span::raw(authors),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            description,
            Style::default()
                .fg(app.theme.fg)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press [i] or [ESC] to close",
            Style::default().fg(app.theme.fg_dim),
        )]),
    ];

    let info_modal = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" About ")
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .style(Style::default().bg(app.theme.bg));

    frame.render_widget(info_modal, modal_area);
}

fn render_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
    let themes = Theme::all();
    let total = themes.len();

    // Modal size: enough for all themes + borders + title + help text
    let modal_width: u16 = 40;
    let modal_height: u16 = (total as u16 + 4).min(area.height.saturating_sub(4));
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

    // The preview theme is the one currently highlighted (for live preview, app.theme
    // is already set to this in the event loop).
    let preview_theme = &app.theme;

    // Build the list items
    let items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, theme)| {
            let is_selected = i == app.theme_picker_index;
            let marker = if is_selected { "\u{25b6} " } else { "  " };
            let label = format!("{}{}", marker, theme.name);
            let style = if is_selected {
                Style::default()
                    .fg(preview_theme.bg)
                    .bg(preview_theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(preview_theme.fg)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let help_line = Line::from(vec![Span::styled(
        " [\u{2191}\u{2193}] Navigate  [Enter] Apply  [Esc] Cancel",
        Style::default().fg(preview_theme.fg_dim),
    )]);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Theme ")
                .title_bottom(help_line)
                .border_style(Style::default().fg(preview_theme.accent)),
        )
        .style(Style::default().bg(preview_theme.bg));

    frame.render_widget(list, modal_area);
}
