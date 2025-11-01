use crate::ui::app::{App, AppState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(frame.area());
    
    render_sidebar(frame, app, chunks[0]);
    
    // Split content area into details and output sections if there's output
    if !app.output.is_empty() {
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);
        
        render_content(frame, app, content_chunks[0]);
        render_output(frame, app, content_chunks[1]);
    } else {
        render_content(frame, app, chunks[1]);
    }
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = match app.state {
        AppState::MainMenu => {
            // Show categories
            let categories = app.categories();
            categories
                .iter()
                .enumerate()
                .map(|(i, cat)| {
                    let icon = match cat.as_str() {
                        "System Management" => "🖥️",
                        "Homelab Operations" => "🏠",
                        "Utilities" => "🛠️",
                        _ => "📁",
                    };
                    let content = format!("{} {}", icon, cat);
                    let style = if i == app.selected_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(content).style(style)
                })
                .collect()
        }
        _ => {
            // Show functions
            let functions = app.filtered_functions();
            functions
                .iter()
                .enumerate()
                .map(|(i, func)| {
                    let style = if i == app.selected_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(func.display_name.clone()).style(style)
                })
                .collect()
        }
    };
    
    let title = match app.state {
        AppState::MainMenu => "╣ JARVIS - Categories ╠",
        _ => "╣ Functions ╠",
    };
    
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));
    
    frame.render_widget(list, area);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.state {
        AppState::MainMenu => {
            let categories = app.categories();
            if let Some(category) = categories.get(app.selected_index) {
                let count = app
                    .functions
                    .iter()
                    .filter(|f| &f.category == category)
                    .count();
                
                let text = vec![
                    Line::from(vec![
                        Span::styled(
                            category.clone(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(format!("Available functions: {}", count)),
                    Line::from(""),
                    Line::from("Press Enter to view functions"),
                    Line::from("Press Q to quit"),
                ];
                
                let paragraph = Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("╣ Details ╠")
                            .border_style(Style::default().fg(Color::Green)),
                    )
                    .wrap(Wrap { trim: true });
                
                frame.render_widget(paragraph, area);
            }
        }
        AppState::CategoryView | AppState::Executing | AppState::ViewingOutput => {
            if let Some(func) = app.selected_function() {
                let text = vec![
                    Line::from(vec![
                        Span::styled(
                            func.display_name.clone(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Category: ", Style::default().fg(Color::Gray)),
                        Span::raw(&func.category),
                    ]),
                    Line::from(vec![
                        Span::styled("Function: ", Style::default().fg(Color::Gray)),
                        Span::raw(&func.name),
                    ]),
                    Line::from(""),
                    Line::from(func.description.clone()),
                    Line::from(""),
                    Line::from("━━━━━━━━━━━━━━━━━━━━━━━━━━━"),
                    Line::from(""),
                    Line::from("[Enter] Execute  [Backspace] Back  [Q] Quit"),
                ];
                
                let paragraph = Paragraph::new(text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("╣ Function Details ╠")
                            .border_style(Style::default().fg(Color::Green)),
                    )
                    .wrap(Wrap { trim: true });
                
                frame.render_widget(paragraph, area);
            }
        }
    }
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
                .title("╣ Last Output ╠")
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: true });
    
    frame.render_widget(paragraph, area);
}
