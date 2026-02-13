//! # Embedded Terminal Widget
//!
//! A ratatui widget that renders the contents of a vt100 virtual terminal
//! screen into the TUI. This is used to display PTY command output inline
//! in the right panel.
//!
//! ## Features
//!
//! - Full ANSI color support (foreground and background)
//! - Bold, italic, underline, inverse attribute support
//! - Scrollback buffer navigation
//! - Visual selection mode for text copying
//!
//! ## Scrollback Design
//!
//! The vt100 crate uses a `set_scrollback(n)` / `scrollback()` API where
//! setting scrollback to N means N rows of history are shown at the top of
//! the visible area, pushing the bottom N screen rows out of view. The
//! `cell(row, col)` method accesses the "visible" rows (scrollback + screen
//! rows combined). We use this mechanism directly: our `scroll_offset`
//! (0 = at bottom) maps to `set_scrollback(scroll_offset)`.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::sync::{Arc, Mutex};

/// Convert a vt100 color to a ratatui Color
fn vt100_color_to_ratatui(color: vt100::Color) -> Option<Color> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(idx) => Some(Color::Indexed(idx)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}

/// A widget that renders a vt100 screen into a ratatui buffer.
///
/// Uses vt100's built-in scrollback mechanism: setting
/// `set_scrollback(offset)` changes what `cell()` returns, so we
/// temporarily adjust the scrollback offset during rendering and
/// restore it afterwards.
pub struct TerminalView<'a> {
    parser: &'a Arc<Mutex<vt100::Parser>>,
    scroll_offset: usize,
    /// Whether visual selection mode is active
    selection_active: bool,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
}

impl<'a> TerminalView<'a> {
    pub fn new(parser: &'a Arc<Mutex<vt100::Parser>>) -> Self {
        Self {
            parser,
            scroll_offset: 0,
            selection_active: false,
            selection_start: None,
            selection_end: None,
        }
    }

    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn selection(
        mut self,
        active: bool,
        start: Option<(usize, usize)>,
        end: Option<(usize, usize)>,
    ) -> Self {
        self.selection_active = active;
        self.selection_start = start;
        self.selection_end = end;
        self
    }

    /// Check if a cell position is within the selection range.
    /// Positions are in the visible coordinate space (row, col).
    fn is_selected(&self, row: usize, col: usize) -> bool {
        if !self.selection_active {
            return false;
        }

        let start = match self.selection_start {
            Some(s) => s,
            None => return false,
        };
        let end = match self.selection_end {
            Some(e) => e,
            None => return false,
        };

        // Normalize so start <= end
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };

        if row < start.0 || row > end.0 {
            return false;
        }
        if row == start.0 && row == end.0 {
            return col >= start.1 && col <= end.1;
        }
        if row == start.0 {
            return col >= start.1;
        }
        if row == end.0 {
            return col <= end.1;
        }
        true
    }
}

impl Widget for TerminalView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut parser = match self.parser.lock() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Save the original scrollback offset and set our desired offset
        let original_scrollback = parser.screen().scrollback();
        parser.screen_mut().set_scrollback(self.scroll_offset);

        // Read screen dimensions
        let (screen_rows, screen_cols) = parser.screen().size();
        let visible_rows = area.height as usize;
        let visible_cols = area.width as usize;

        // Collect cell data into a temporary buffer to avoid borrow conflicts.
        // Each entry: (display_y, display_x, contents, style)
        let mut cells: Vec<(u16, u16, String, Style)> = Vec::new();

        {
            let screen = parser.screen();
            for display_y in 0..visible_rows.min(screen_rows as usize) {
                for display_x in 0..visible_cols.min(screen_cols as usize) {
                    let buf_x = area.x + display_x as u16;
                    let buf_y = area.y + display_y as u16;

                    if buf_x >= area.x + area.width || buf_y >= area.y + area.height {
                        continue;
                    }

                    if let Some(cell) = screen.cell(display_y as u16, display_x as u16) {
                        let ch = cell.contents();
                        let ch = if ch.is_empty() { " " } else { ch };

                        let mut style = Style::default();

                        if let Some(fg) = vt100_color_to_ratatui(cell.fgcolor()) {
                            style = style.fg(fg);
                        }
                        if let Some(bg) = vt100_color_to_ratatui(cell.bgcolor()) {
                            style = style.bg(bg);
                        }
                        if cell.bold() {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        if cell.italic() {
                            style = style.add_modifier(Modifier::ITALIC);
                        }
                        if cell.underline() {
                            style = style.add_modifier(Modifier::UNDERLINED);
                        }
                        if cell.inverse() {
                            style = style.add_modifier(Modifier::REVERSED);
                        }

                        // Apply selection highlight
                        if self.is_selected(display_y, display_x) {
                            style = style.bg(Color::Blue).fg(Color::White);
                        }

                        cells.push((buf_x, buf_y, ch.to_string(), style));
                    }
                }
            }
        }

        // Write collected cells to the buffer
        for (x, y, ch, style) in cells {
            buf.set_string(x, y, &ch, style);
        }

        // Restore the original scrollback offset
        parser.screen_mut().set_scrollback(original_scrollback);
    }
}

/// Get the text contents of a selection range from the vt100 screen.
///
/// Start and end are `(row, col)` pairs in the visible coordinate space
/// (i.e., relative to the current scrollback view).
pub fn get_selected_text(
    parser: &Arc<Mutex<vt100::Parser>>,
    scroll_offset: usize,
    start: (usize, usize),
    end: (usize, usize),
) -> String {
    let mut parser = match parser.lock() {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    // Set scrollback to match the view the user is seeing
    let original_scrollback = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(scroll_offset);

    let (screen_rows, screen_cols) = parser.screen().size();

    // Normalize
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let mut text = String::new();

    {
        let screen = parser.screen();
        for row in start.0..=end.0.min(screen_rows as usize - 1) {
            let col_start = if row == start.0 { start.1 } else { 0 };
            let col_end = if row == end.0 {
                end.1
            } else {
                screen_cols as usize - 1
            };

            for col in col_start..=col_end.min(screen_cols as usize - 1) {
                if let Some(cell) = screen.cell(row as u16, col as u16) {
                    let contents = cell.contents();
                    text.push_str(if contents.is_empty() { " " } else { contents });
                }
            }

            if row < end.0 {
                text.push('\n');
            }
        }
    }

    // Restore scrollback
    parser.screen_mut().set_scrollback(original_scrollback);

    // Trim trailing whitespace from each line
    text.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

/// Get the maximum number of scrollback rows available.
///
/// This temporarily sets scrollback to `usize::MAX` to find how many
/// scrollback rows actually exist, then restores the original value.
pub fn max_scrollback(parser: &Arc<Mutex<vt100::Parser>>) -> usize {
    let mut parser = match parser.lock() {
        Ok(p) => p,
        Err(_) => return 0,
    };
    let original = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(usize::MAX);
    let max = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(original);
    max
}

/// Get the total number of content lines (scrollback + screen).
pub fn total_content_lines(parser: &Arc<Mutex<vt100::Parser>>) -> usize {
    let mut parser = match parser.lock() {
        Ok(p) => p,
        Err(_) => return 0,
    };
    let (rows, _) = parser.screen().size();
    let original = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(usize::MAX);
    let max_scrollback = parser.screen().scrollback();
    parser.screen_mut().set_scrollback(original);
    max_scrollback + rows as usize
}
