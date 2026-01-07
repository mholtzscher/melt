//! Common rendering utilities

use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme;

/// Spinner animation frames
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Get the current spinner frame based on tick count
pub fn get_spinner_frame(tick: u64) -> &'static str {
    SPINNER_FRAMES[(tick as usize / 2) % SPINNER_FRAMES.len()]
}

/// Render loading screen
pub fn render_loading(frame: &mut Frame, message: &str, tick_count: u64) {
    let area = frame.area();
    let spinner = get_spinner_frame(tick_count);

    let text = vec![
        Line::from(vec![
            Span::styled(spinner, Style::default().fg(theme::ACCENT)),
            Span::styled(format!(" {}", message), Style::default().fg(theme::TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press q or Ctrl+C to cancel",
            Style::default().fg(theme::TEXT_DIM),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);

    let chunks = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(3),
        Constraint::Percentage(40),
    ])
    .split(area);

    frame.render_widget(paragraph, chunks[1]);
}

/// Render error screen
pub fn render_error(frame: &mut Frame, error: &str) {
    let area = frame.area();

    let text = vec![
        Line::from(Span::styled(
            format!("Error: {}", error),
            Style::default().fg(theme::ERROR),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to exit",
            Style::default().fg(theme::TEXT_DIM),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);

    let chunks = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(3),
        Constraint::Percentage(40),
    ])
    .split(area);

    frame.render_widget(paragraph, chunks[1]);
}
