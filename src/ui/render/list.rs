//! List view rendering

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::app::state::ListState;
use crate::model::{FlakeInput, StatusLevel, StatusMessage, UpdateStatus};
use crate::ui::theme;
use crate::util::time::format_relative;

use super::common::get_spinner_frame;

/// Render the list view
pub fn render_list(
    frame: &mut Frame,
    list: &mut ListState,
    status_message: Option<&StatusMessage>,
    tick_count: u64,
) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);

    render_input_table(frame, list, chunks[0], tick_count);
    render_help_bar(frame, list, status_message, chunks[1], tick_count);
}

/// Render the input table
fn render_input_table(frame: &mut Frame, list: &mut ListState, area: Rect, tick_count: u64) {
    let header = Row::new(vec![" ", "NAME", "TYPE", "REV", "UPDATED", "STATUS"])
        .style(Style::default().fg(theme::TEXT_DIM));

    let rows: Vec<Row> = list
        .flake
        .inputs
        .iter()
        .enumerate()
        .map(|(idx, input)| {
            let is_selected = list.selected.contains(&idx);
            let checkbox = if is_selected { "[x]" } else { "[ ]" };
            let checkbox_style = if is_selected {
                Style::default()
                    .fg(theme::SELECTED)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_DIM)
            };

            let type_color = match input {
                FlakeInput::Git(_) => theme::TYPE_GIT,
                FlakeInput::Path(_) => theme::TYPE_PATH,
                FlakeInput::Other(_) => theme::TYPE_OTHER,
            };

            let status = list
                .update_statuses
                .get(input.name())
                .cloned()
                .unwrap_or_default();

            let status_display = match &status {
                UpdateStatus::Checking => get_spinner_frame(tick_count).to_string(),
                _ => status.display(),
            };

            let status_color = match &status {
                UpdateStatus::Unknown => theme::TEXT_DIM,
                UpdateStatus::Checking => theme::TEXT_DIM,
                UpdateStatus::UpToDate => theme::TEXT_DIM,
                UpdateStatus::Behind(_) => theme::SUCCESS,
                UpdateStatus::Error(_) => theme::WARNING,
            };

            Row::new(vec![
                Span::styled(checkbox, checkbox_style),
                Span::styled(input.name(), Style::default().fg(theme::TEXT)),
                Span::styled(input.type_display(), Style::default().fg(type_color)),
                Span::styled(
                    input.short_rev().unwrap_or("-"),
                    Style::default().fg(theme::ACCENT),
                ),
                Span::styled(
                    input
                        .last_modified()
                        .map(format_relative)
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(theme::TEXT_MUTED),
                ),
                Span::styled(status_display, Style::default().fg(status_color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(5),
        Constraint::Length(35),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Min(6),
    ];

    let title = list.flake.path.to_string_lossy();
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(format!(" {} ", title))
                .title_style(Style::default().fg(theme::TEXT)),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme::BG_HIGHLIGHT)
                .fg(theme::CURSOR)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(table, area, &mut list.table_state);
}

/// Render the help bar
fn render_help_bar(
    frame: &mut Frame,
    list: &ListState,
    status_message: Option<&StatusMessage>,
    area: Rect,
    tick_count: u64,
) {
    let shortcuts = vec![
        ("j/k", "nav"),
        ("space", "select"),
        ("u", "update"),
        ("U", "all"),
        ("c", "changelog"),
        ("r", "refresh"),
        ("q", "quit"),
    ];

    let mut spans: Vec<Span> = shortcuts
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Style::default().fg(theme::KEY_HINT)),
                Span::styled(format!(" {} ", desc), Style::default().fg(theme::TEXT_DIM)),
            ]
        })
        .collect();

    if !list.selected.is_empty() {
        spans.push(Span::styled(
            format!(" | {} selected", list.selected.len()),
            Style::default().fg(theme::SELECTED),
        ));
    }

    // Show error message for current input if it has an error status
    if let Some(input) = list.flake.inputs.get(list.cursor) {
        if let Some(UpdateStatus::Error(err)) = list.update_statuses.get(input.name()) {
            let truncated = if err.len() > 60 {
                format!("{}...", &err[..57])
            } else {
                err.clone()
            };
            spans.push(Span::styled(
                format!(" | {}", truncated),
                Style::default().fg(theme::ERROR),
            ));
        }
    }

    if let Some(msg) = status_message {
        let color = match msg.level {
            StatusLevel::Info => theme::INFO,
            StatusLevel::Success => theme::SUCCESS,
            StatusLevel::Warning => theme::WARNING,
            StatusLevel::Error => theme::ERROR,
        };
        // Add spinner for info messages (indicates in-progress operation)
        let spinner = if msg.level == StatusLevel::Info {
            format!("{} ", get_spinner_frame(tick_count))
        } else {
            String::new()
        };
        spans.push(Span::styled(
            format!(" | {}{}", spinner, msg.text),
            Style::default().fg(color),
        ));
    }

    let help = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER)),
    );

    frame.render_widget(help, area);
}
