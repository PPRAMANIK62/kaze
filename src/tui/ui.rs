//! TUI drawing logic for kaze.
//!
//! Renders the two-pane layout: a scrollable message history area on top
//! and an auto-growing input box on the bottom.

use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::App;

/// Draws the TUI frame with message history and input box.
///
/// The layout is split vertically:
/// - Top pane: scrollable message history with border and title ` kaze `
/// - Bottom pane (dynamic height): auto-growing input box with border and title ` > `
///
/// The cursor is placed at the end of the current input text.
pub fn draw(f: &mut Frame, app: &App) {
    // Inner width = total area width minus 2 for left/right borders
    let inner_width = f.area().width.saturating_sub(2).max(1) as usize;

    // Number of visual lines the input text occupies when wrapped
    let visual_lines = if app.input.is_empty() {
        1
    } else {
        app.input.len().div_ceil(inner_width).max(1)
    };

    // Input box height = visual lines + 2 (top/bottom borders)
    // Cap at 40% of terminal height to protect the message area
    let max_input_height = (f.area().height as usize * 2 / 5).max(3);
    let input_height = (visual_lines + 2).min(max_input_height) as u16;

    let [messages_area, input_area] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(input_height)]).areas(f.area());

    // --- Message history pane ---
    let mut lines: Vec<Line<'_>> = Vec::new();
    for msg in &app.messages {
        let role_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        lines.push(Line::from(vec![
            Span::styled(format!("[{}]", msg.role), role_style),
            Span::raw(" "),
            Span::raw(&msg.content),
        ]));
        lines.push(Line::from(""));
    }

    // Spinner while waiting for first token
    if app.waiting {
        let spinner = crate::constants::SPINNER_FRAMES[app.spinner_frame];
        lines.push(Line::from(Span::styled(
            format!("{} Thinking...", spinner),
            Style::default().fg(Color::Yellow),
        )));
    }

    // Streaming cursor indicator
    if app.streaming {
        lines.push(Line::from(Span::styled(
            "▍",
            Style::default().fg(Color::Green),
        )));
    }

    let messages_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" kaze "))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));
    f.render_widget(messages_widget, messages_area);

    // --- Input box ---
    let input_widget = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(" > "))
        .wrap(Wrap { trim: false });
    f.render_widget(input_widget, input_area);

    // Place cursor at end of input text, accounting for line wrapping.
    let iw = (input_area.width.saturating_sub(2)).max(1) as usize;
    let len = app.input.len();
    let cursor_x = input_area.x + 1 + (len % iw) as u16;
    let cursor_y = input_area.y + 1 + (len / iw) as u16;
    f.set_cursor_position(Position::new(cursor_x, cursor_y));
}
