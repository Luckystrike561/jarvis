use crate::ui::app::{App, FocusPane, TreeItem};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    // Main layout: Header + Body + Footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(frame.area());

    // Render header
    render_header(frame, main_chunks[0]);

    // Split body into left (scripts) and right (details/output)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[1]);

    // Render script tree on left
    render_script_tree(frame, app, body_chunks[0]);

    // Split right side into details and output
    if !app.output.is_empty() {
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body_chunks[1]);

        render_details(frame, app, right_chunks[0]);
        render_output(frame, app, right_chunks[1]);
    } else {
        render_details(frame, app, body_chunks[1]);
    }

    // Render footer
    render_footer(frame, app, main_chunks[2]);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let header_text = vec![Line::from(vec![Span::styled(
        "  JARVIS v2.0 - Just Another Rather Very Intelligent System  ",
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

fn render_script_tree(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .tree_items()
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.selected_index;
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
                    let cat_icon = match category.as_str() {
                        "System Management" => "🖥️",
                        "Homelab Operations" => "🏠",
                        "Utilities" => "🛠️",
                        _ => "📁",
                    };
                    let content = format!("{} {} {}", icon, cat_icon, category);
                    ListItem::new(content).style(style)
                }
                TreeItem::Function(func) => {
                    let content = format!("    {}", func.display_name);
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

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📁 Scripts")
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

            vec![
                Line::from(vec![Span::styled(
                    category.clone(),
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

fn render_output(frame: &mut Frame, app: &App, area: Rect) {
    let text: Vec<Line> = app
        .output
        .iter()
        .map(|line| Line::from(line.clone()))
        .collect();

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("💬 Output")
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.focus {
        FocusPane::ScriptList => {
            "[↑↓/jk] Navigate  [←→/hl] Collapse/Expand  [Enter] Toggle/Execute  [Tab] Switch  [Q] Quit"
        }
        FocusPane::Details => "[Tab] Switch Pane  [Q] Quit",
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default());

    frame.render_widget(footer, area);
}
