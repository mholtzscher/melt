//! Changelog view rendering

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table},
    Frame,
};

use crate::app::state::ChangelogState;
use crate::model::{StatusLevel, StatusMessage};
use crate::ui::theme;
use crate::util::time::format_relative_short;

/// Render the changelog view
pub fn render_changelog(
    frame: &mut Frame,
    cs: &mut ChangelogState,
    status_message: Option<&StatusMessage>,
) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);

    render_commits_table(frame, cs, chunks[0]);
    render_changelog_help_bar(frame, cs, status_message, chunks[1]);

    if cs.confirm_lock.is_some() {
        render_confirm_dialog(frame, cs, area);
    }
}

/// Render the commits table
fn render_commits_table(frame: &mut Frame, cs: &mut ChangelogState, area: Rect) {
    if cs.data.commits.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .title(format!(" {} ({}) ", cs.input.name, cs.input.url))
            .title_style(Style::default().fg(theme::TEXT));

        let msg = Paragraph::new("Already up to date!")
            .style(Style::default().fg(theme::SUCCESS))
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(msg, area);
        return;
    }

    let rows: Vec<Row> = cs
        .data
        .commits
        .iter()
        .map(|commit| {
            let lock_icon = if commit.is_locked { "🔒" } else { "  " };
            let sha_color = if commit.is_locked {
                theme::WARNING
            } else {
                theme::SHA
            };

            let author = if commit.author.len() > 14 {
                format!("{}...", &commit.author[..12])
            } else {
                format!("{:14}", commit.author)
            };

            let message = if commit.message.len() > 55 {
                format!("{}...", &commit.message[..52])
            } else {
                commit.message.clone()
            };

            Row::new(vec![
                Span::styled(lock_icon, Style::default().fg(theme::WARNING)),
                Span::styled(commit.short_sha(), Style::default().fg(sha_color)),
                Span::styled(author, Style::default().fg(theme::INFO)),
                Span::styled(
                    format_relative_short(commit.date),
                    Style::default().fg(theme::TEXT_DIM),
                ),
                Span::styled(message, Style::default().fg(theme::TEXT)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(3),
        Constraint::Length(9),
        Constraint::Length(16),
        Constraint::Length(10),
        Constraint::Min(20),
    ];

    let title = format!(" {} ({}) ", cs.input.name, cs.input.url);
    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(title)
                .title_style(Style::default().fg(theme::TEXT)),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme::BG_HIGHLIGHT)
                .fg(theme::CURSOR)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(table, area, &mut cs.table_state);
}

/// Render the changelog help bar
fn render_changelog_help_bar(
    frame: &mut Frame,
    cs: &ChangelogState,
    status_message: Option<&StatusMessage>,
    area: Rect,
) {
    let shortcuts = vec![("j/k", "nav"), ("space", "lock"), ("q/esc", "back")];

    let mut spans: Vec<Span> = shortcuts
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Style::default().fg(theme::KEY_HINT)),
                Span::styled(format!(" {} ", desc), Style::default().fg(theme::TEXT_DIM)),
            ]
        })
        .collect();

    if !cs.data.commits.is_empty() {
        let ahead = cs.data.commits_ahead();
        let behind = cs.data.commits_behind();

        spans.push(Span::styled(" | ", Style::default().fg(theme::TEXT_DIM)));
        spans.push(Span::styled(
            format!("+{} new", ahead),
            Style::default().fg(theme::SUCCESS),
        ));
        spans.push(Span::styled(" 🔒 ", Style::default().fg(theme::WARNING)));
        spans.push(Span::styled(
            format!("{} older", behind),
            Style::default().fg(theme::TEXT_MUTED),
        ));
    }

    if let Some(msg) = status_message {
        let color = match msg.level {
            StatusLevel::Info => theme::INFO,
            StatusLevel::Success => theme::SUCCESS,
            StatusLevel::Warning => theme::WARNING,
            StatusLevel::Error => theme::ERROR,
        };
        spans.push(Span::styled(
            format!(" | {}", msg.text),
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

/// Render the confirmation dialog
fn render_confirm_dialog(frame: &mut Frame, cs: &ChangelogState, area: Rect) {
    let commit_idx = match cs.confirm_lock {
        Some(idx) => idx,
        None => return,
    };

    let commit = match cs.data.commits.get(commit_idx) {
        Some(c) => c,
        None => return,
    };

    let dialog_width = 50;
    let dialog_height = 7;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let msg_preview = if commit.message.len() > 40 {
        format!("{}...", &commit.message[..37])
    } else {
        commit.message.clone()
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Lock ", Style::default().fg(theme::TEXT)),
            Span::styled(
                &cs.input.name,
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to ", Style::default().fg(theme::TEXT)),
            Span::styled(
                commit.short_sha(),
                Style::default().fg(theme::SHA).add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            msg_preview,
            Style::default().fg(theme::TEXT_DIM),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(theme::SUCCESS)),
            Span::styled(" confirm  ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("n/q", Style::default().fg(theme::ERROR)),
            Span::styled(" cancel", Style::default().fg(theme::TEXT_DIM)),
        ]),
    ];

    let dialog = Paragraph::new(text).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT))
            .style(Style::default().bg(theme::BG_DARK)),
    );

    frame.render_widget(dialog, dialog_area);
}
