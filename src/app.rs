use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState},
    Frame,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    error::AppResult,
    event::{poll_key, KeyEventExt},
    model::{ChangelogData, FlakeData, FlakeInput, GitInput, StatusMessage, UpdateStatus},
    service::{GitService, NixService},
    tui::Tui,
    ui::theme,
    util::time::{format_relative, format_relative_short},
};

/// Messages from background tasks
enum TaskResult {
    FlakeLoaded(Result<FlakeData, String>),
    UpdateComplete(Result<(), String>),
    ChangelogLoaded(Result<(GitInput, usize, ChangelogData, ListState), String>),
    LockComplete(Result<(), String>),
    /// Status update for a single input
    InputStatus { name: String, status: UpdateStatus },
}

/// Main application struct
pub struct App {
    /// Path to the flake
    flake_path: PathBuf,
    /// Current state
    state: AppState,
    /// Nix service
    nix: NixService,
    /// Git service
    git: GitService,
    /// Cancellation token for async operations
    cancel_token: CancellationToken,
    /// Status message to display
    status_message: Option<StatusMessage>,
    /// Tick count for animations
    tick_count: u64,
    /// Channel for receiving task results
    task_rx: mpsc::UnboundedReceiver<TaskResult>,
    /// Channel for sending task results
    task_tx: mpsc::UnboundedSender<TaskResult>,
}

/// Application state machine
pub enum AppState {
    /// Loading flake metadata
    Loading,
    /// Error occurred
    Error(String),
    /// Showing list of inputs
    List(ListState),
    /// Showing changelog for an input
    Changelog(ChangelogState),
    /// Loading changelog (keep parent list for display)
    LoadingChangelog(ListState),
    /// Quitting
    Quitting,
}

/// Simple enum for state discrimination without borrowing
#[derive(Clone, Copy)]
enum StateKind {
    Loading,
    Error,
    List,
    Changelog,
    LoadingChangelog,
    Quitting,
}

/// State for the list view
pub struct ListState {
    pub flake: FlakeData,
    pub cursor: usize,
    pub selected: std::collections::HashSet<usize>,
    pub table_state: TableState,
    pub update_statuses: HashMap<String, UpdateStatus>,
    /// True when a background operation is in progress
    pub busy: bool,
}

impl Clone for ListState {
    fn clone(&self) -> Self {
        Self {
            flake: self.flake.clone(),
            cursor: self.cursor,
            selected: self.selected.clone(),
            table_state: TableState::default().with_selected(self.table_state.selected()),
            update_statuses: self.update_statuses.clone(),
            busy: self.busy,
        }
    }
}

/// State for the changelog view
pub struct ChangelogState {
    /// The input we're showing changelog for
    pub input: GitInput,
    /// Index of this input in the parent list
    pub input_idx: usize,
    /// The changelog data
    pub data: ChangelogData,
    /// Current cursor position
    pub cursor: usize,
    /// Table state for rendering
    pub table_state: TableState,
    /// If Some, show confirm dialog for locking to this commit index
    pub confirm_lock: Option<usize>,
    /// Parent list state (kept for returning)
    pub parent_list: ListState,
}

impl App {
    /// Create a new application instance
    pub fn new(flake_path: PathBuf) -> Self {
        let cancel_token = CancellationToken::new();
        let (task_tx, task_rx) = mpsc::unbounded_channel();
        Self {
            flake_path,
            state: AppState::Loading,
            nix: NixService::new(cancel_token.clone()),
            git: GitService::new(cancel_token.clone()),
            cancel_token,
            status_message: None,
            tick_count: 0,
            task_rx,
            task_tx,
        }
    }

    /// Run the application main loop
    pub async fn run(&mut self, tui: &mut Tui) -> AppResult<()> {
        // Start loading flake in background
        self.spawn_load_flake();

        loop {
            // Check for quit state
            if matches!(self.state, AppState::Quitting) {
                break;
            }

            // Draw the UI
            tui.draw(|frame| self.render(frame))?;

            // Poll for key events (non-blocking with short timeout)
            if let Some(key) = poll_key(Duration::from_millis(16)) {
                self.handle_key(key).await;
            }

            // Check for background task results (non-blocking)
            while let Ok(result) = self.task_rx.try_recv() {
                self.handle_task_result(result);
            }

            // Increment tick for animations
            self.tick_count = self.tick_count.wrapping_add(1);

            // Clear expired status messages
            if let Some(ref msg) = self.status_message {
                if msg.is_expired() {
                    self.status_message = None;
                }
            }
        }

        Ok(())
    }

    /// Spawn a background task to load flake metadata
    fn spawn_load_flake(&self) {
        let nix = self.nix.clone();
        let path = self.flake_path.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.load_metadata(&path).await;
            let _ = tx.send(TaskResult::FlakeLoaded(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    /// Spawn a background task to update inputs
    fn spawn_update(&self, path: PathBuf, names: Vec<String>) {
        let nix = self.nix.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.update_inputs(&path, &names).await;
            let _ = tx.send(TaskResult::UpdateComplete(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    /// Spawn a background task to update all inputs
    fn spawn_update_all(&self, path: PathBuf) {
        let nix = self.nix.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.update_all(&path).await;
            let _ = tx.send(TaskResult::UpdateComplete(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    /// Spawn a background task to load changelog
    fn spawn_load_changelog(&self, input: GitInput, input_idx: usize, parent_list: ListState) {
        let git = self.git.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = git.get_changelog(&input).await;
            let _ = tx.send(TaskResult::ChangelogLoaded(
                result
                    .map(|data| (input, input_idx, data, parent_list))
                    .map_err(|e| e.to_string()),
            ));
        });
    }

    /// Spawn a background task to lock an input
    fn spawn_lock(&self, path: PathBuf, name: String, lock_url: String) {
        let nix = self.nix.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.lock_input(&path, &name, &lock_url).await;
            let _ = tx.send(TaskResult::LockComplete(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    /// Spawn background tasks to check for updates on all inputs
    fn spawn_check_updates(&self, inputs: Vec<FlakeInput>) {
        let git = self.git.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let _ = git.check_updates(&inputs, |name, status| {
                let _ = tx.send(TaskResult::InputStatus {
                    name: name.to_string(),
                    status,
                });
            }).await;
        });
    }

    /// Handle a result from a background task
    fn handle_task_result(&mut self, result: TaskResult) {
        match result {
            TaskResult::FlakeLoaded(Ok(flake)) => {
                let inputs = flake.inputs.clone();
                
                // Check if we're refreshing (already in List state) or initial load
                if let AppState::List(list) = &mut self.state {
                    // Refresh: update flake data, keep cursor/selection, clear busy
                    list.flake = flake;
                    list.busy = false;
                    // Clamp cursor to new input count
                    if list.cursor >= list.flake.inputs.len() {
                        list.cursor = list.flake.inputs.len().saturating_sub(1);
                        list.table_state.select(Some(list.cursor));
                    }
                    // Clear selections that are now out of bounds
                    list.selected.retain(|&i| i < list.flake.inputs.len());
                    // Clear old update statuses
                    list.update_statuses.clear();
                } else {
                    // Initial load: create fresh state
                    let mut table_state = TableState::default();
                    if !flake.inputs.is_empty() {
                        table_state.select(Some(0));
                    }
                    self.state = AppState::List(ListState {
                        flake,
                        cursor: 0,
                        selected: std::collections::HashSet::new(),
                        table_state,
                        update_statuses: HashMap::new(),
                        busy: false,
                    });
                }
                
                // Clear any status message (e.g., "Refreshing...")
                self.status_message = None;
                // Start checking for updates in background
                self.spawn_check_updates(inputs);
            }
            TaskResult::FlakeLoaded(Err(e)) => {
                self.state = AppState::Error(format!("Failed to load flake: {}", e));
            }
            TaskResult::UpdateComplete(Ok(())) => {
                self.status_message = Some(StatusMessage::success("Update complete"));
                // Clear selection and reload (keep list visible)
                if let AppState::List(list) = &mut self.state {
                    list.selected.clear();
                    // Keep busy=true until FlakeLoaded arrives
                }
                self.spawn_load_flake();
            }
            TaskResult::UpdateComplete(Err(e)) => {
                self.status_message = Some(StatusMessage::error(format!("Update failed: {}", e)));
                // Clear busy flag
                if let AppState::List(list) = &mut self.state {
                    list.busy = false;
                }
            }
            TaskResult::ChangelogLoaded(Ok((input, input_idx, data, parent_list))) => {
                let cursor = data.locked_idx.unwrap_or(0);
                let mut table_state = TableState::default();
                if !data.commits.is_empty() {
                    table_state.select(Some(cursor));
                }
                self.state = AppState::Changelog(ChangelogState {
                    input,
                    input_idx,
                    data,
                    cursor,
                    table_state,
                    confirm_lock: None,
                    parent_list,
                });
                self.status_message = None;
            }
            TaskResult::ChangelogLoaded(Err(e)) => {
                self.status_message = Some(StatusMessage::error(format!("Failed to load changelog: {}", e)));
                // Return to list from loading changelog state
                if let AppState::LoadingChangelog(list) = std::mem::replace(&mut self.state, AppState::Loading) {
                    self.state = AppState::List(list);
                }
            }
            TaskResult::LockComplete(Ok(())) => {
                self.status_message = Some(StatusMessage::success("Locked successfully"));
                // Return to list and reload (keep list visible)
                if let AppState::Changelog(cs) = std::mem::replace(&mut self.state, AppState::Loading) {
                    let mut list = cs.parent_list;
                    list.busy = true;
                    self.state = AppState::List(list);
                }
                self.spawn_load_flake();
            }
            TaskResult::LockComplete(Err(e)) => {
                self.status_message = Some(StatusMessage::error(format!("Lock failed: {}", e)));
                // Clear confirm dialog if in changelog
                if let AppState::Changelog(cs) = &mut self.state {
                    cs.confirm_lock = None;
                }
            }
            TaskResult::InputStatus { name, status } => {
                // Update the status for this input
                if let AppState::List(list) = &mut self.state {
                    list.update_statuses.insert(name, status);
                }
            }
        }
    }

    /// Handle a key event
    async fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        let state_kind = match &self.state {
            AppState::Loading => StateKind::Loading,
            AppState::Error(_) => StateKind::Error,
            AppState::List(_) => StateKind::List,
            AppState::Changelog(_) => StateKind::Changelog,
            AppState::LoadingChangelog(_) => StateKind::LoadingChangelog,
            AppState::Quitting => StateKind::Quitting,
        };

        match state_kind {
            StateKind::Loading | StateKind::LoadingChangelog => {
                if key.is_quit() {
                    self.cancel_token.cancel();
                    self.state = AppState::Quitting;
                }
            }
            StateKind::Error => {
                self.state = AppState::Quitting;
            }
            StateKind::List => {
                self.handle_list_key(key);
            }
            StateKind::Changelog => {
                self.handle_changelog_key(key);
            }
            StateKind::Quitting => {}
        }
    }

    /// Handle key events in list view
    fn handle_list_key(&mut self, key: crossterm::event::KeyEvent) {
        let (input_count, has_selection, is_busy) = match &self.state {
            AppState::List(list) => (list.flake.inputs.len(), !list.selected.is_empty(), list.busy),
            _ => return,
        };

        if input_count == 0 {
            if key.is_quit() {
                self.state = AppState::Quitting;
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if has_selection {
                    if let AppState::List(list) = &mut self.state {
                        list.selected.clear();
                    }
                } else {
                    self.state = AppState::Quitting;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let AppState::List(list) = &mut self.state {
                    if list.cursor < input_count - 1 {
                        list.cursor += 1;
                        list.table_state.select(Some(list.cursor));
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let AppState::List(list) = &mut self.state {
                    if list.cursor > 0 {
                        list.cursor -= 1;
                        list.table_state.select(Some(list.cursor));
                    }
                }
            }
            KeyCode::Char(' ') => {
                if !is_busy {
                    if let AppState::List(list) = &mut self.state {
                        let cursor = list.cursor;
                        if list.selected.contains(&cursor) {
                            list.selected.remove(&cursor);
                        } else {
                            list.selected.insert(cursor);
                        }
                    }
                }
            }
            KeyCode::Char('u') => {
                if is_busy {
                    return;
                }
                // Update selected inputs
                if let AppState::List(list) = &mut self.state {
                    let names: Vec<String> = list
                        .selected
                        .iter()
                        .filter_map(|&i| list.flake.inputs.get(i))
                        .map(|input| input.name().to_string())
                        .collect();

                    if !names.is_empty() {
                        let path = list.flake.path.clone();
                        let count = names.len();
                        list.busy = true;
                        self.status_message = Some(StatusMessage::info(format!(
                            "Updating {} input(s)...",
                            count
                        )));
                        self.spawn_update(path, names);
                    } else {
                        self.status_message = Some(StatusMessage::warning("No inputs selected"));
                    }
                }
            }
            KeyCode::Char('U') => {
                if is_busy {
                    return;
                }
                // Update all
                if let AppState::List(list) = &mut self.state {
                    let path = list.flake.path.clone();
                    list.busy = true;
                    self.status_message = Some(StatusMessage::info("Updating all inputs..."));
                    self.spawn_update_all(path);
                }
            }
            KeyCode::Char('r') => {
                if is_busy {
                    return;
                }
                // Refresh - keep list visible, just show status
                if let AppState::List(list) = &mut self.state {
                    list.busy = true;
                }
                self.status_message = Some(StatusMessage::info("Refreshing..."));
                self.spawn_load_flake();
            }
            KeyCode::Char('c') => {
                if is_busy {
                    return;
                }
                // Open changelog
                if let AppState::List(list) = &self.state {
                    let idx = list.cursor;
                    if let Some(FlakeInput::Git(git_input)) = list.flake.inputs.get(idx) {
                        let input = git_input.clone();
                        let mut parent = list.clone();
                        parent.busy = false; // Reset busy for parent
                        self.status_message = Some(StatusMessage::info("Loading changelog..."));
                        self.state = AppState::LoadingChangelog(parent.clone());
                        self.spawn_load_changelog(input, idx, parent);
                    } else {
                        self.status_message = Some(StatusMessage::warning(
                            "Changelog only available for git inputs",
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle key events in changelog view
    fn handle_changelog_key(&mut self, key: crossterm::event::KeyEvent) {
        // Check if we're in confirm dialog
        let in_confirm = match &self.state {
            AppState::Changelog(cs) => cs.confirm_lock.is_some(),
            _ => return,
        };

        if in_confirm {
            self.handle_confirm_key(key);
            return;
        }

        let commit_count = match &self.state {
            AppState::Changelog(cs) => cs.data.commits.len(),
            _ => return,
        };

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.close_changelog();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let AppState::Changelog(cs) = &mut self.state {
                    if cs.cursor < commit_count.saturating_sub(1) {
                        cs.cursor += 1;
                        cs.table_state.select(Some(cs.cursor));
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let AppState::Changelog(cs) = &mut self.state {
                    if cs.cursor > 0 {
                        cs.cursor -= 1;
                        cs.table_state.select(Some(cs.cursor));
                    }
                }
            }
            KeyCode::Char(' ') => {
                if let AppState::Changelog(cs) = &mut self.state {
                    if !cs.data.commits.is_empty() {
                        cs.confirm_lock = Some(cs.cursor);
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle key events in confirm dialog
    fn handle_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('y') => {
                self.confirm_lock();
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                if let AppState::Changelog(cs) = &mut self.state {
                    cs.confirm_lock = None;
                }
            }
            _ => {}
        }
    }

    /// Confirm locking to a commit
    fn confirm_lock(&mut self) {
        let (input, commit_sha, flake_path) = match &self.state {
            AppState::Changelog(cs) => {
                let commit_idx = match cs.confirm_lock {
                    Some(idx) => idx,
                    None => return,
                };
                let commit = match cs.data.commits.get(commit_idx) {
                    Some(c) => c,
                    None => return,
                };
                (cs.input.clone(), commit.sha.clone(), cs.parent_list.flake.path.clone())
            }
            _ => return,
        };

        let lock_url = input.forge_type.lock_url(
            &input.owner,
            &input.repo,
            &commit_sha,
            input.host.as_deref(),
        );

        if lock_url.is_empty() {
            self.status_message = Some(StatusMessage::error("Cannot generate lock URL for this input"));
            if let AppState::Changelog(cs) = &mut self.state {
                cs.confirm_lock = None;
            }
            return;
        }

        self.status_message = Some(StatusMessage::info(format!(
            "Locking {} to {}...",
            input.name,
            &commit_sha[..7.min(commit_sha.len())]
        )));

        self.spawn_lock(flake_path, input.name.clone(), lock_url);
    }

    /// Close changelog and return to list
    fn close_changelog(&mut self) {
        if let AppState::Changelog(cs) = std::mem::replace(&mut self.state, AppState::Loading) {
            self.state = AppState::List(cs.parent_list);
        }
    }

    /// Render the application UI
    fn render(&mut self, frame: &mut Frame) {
        match &mut self.state {
            AppState::Loading => self.render_loading(frame, "Loading flake..."),
            AppState::Error(msg) => {
                let msg = msg.clone();
                self.render_error(frame, &msg);
            }
            AppState::List(list) => {
                let status_message = self.status_message.as_ref();
                let tick_count = self.tick_count;
                render_list(frame, list, status_message, tick_count);
            }
            AppState::LoadingChangelog(list) => {
                // Show list with loading message
                let status_message = self.status_message.as_ref();
                let tick_count = self.tick_count;
                render_list(frame, list, status_message, tick_count);
            }
            AppState::Changelog(cs) => {
                let status_message = self.status_message.as_ref();
                render_changelog(frame, cs, status_message);
            }
            AppState::Quitting => {}
        }
    }

    /// Render loading screen
    fn render_loading(&self, frame: &mut Frame, message: &str) {
        let area = frame.area();
        let spinner = get_spinner_frame(self.tick_count);

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
    fn render_error(&self, frame: &mut Frame, error: &str) {
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
}

/// Render the list view
fn render_list(
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

    if let Some(msg) = status_message {
        let color = match msg.level {
            crate::model::StatusLevel::Info => theme::INFO,
            crate::model::StatusLevel::Success => theme::SUCCESS,
            crate::model::StatusLevel::Warning => theme::WARNING,
            crate::model::StatusLevel::Error => theme::ERROR,
        };
        // Add spinner for info messages (indicates in-progress operation)
        let spinner = if msg.level == crate::model::StatusLevel::Info {
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

/// Get the current spinner frame
fn get_spinner_frame(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[(tick as usize / 2) % FRAMES.len()]
}

/// Render the changelog view
fn render_changelog(frame: &mut Frame, cs: &mut ChangelogState, status_message: Option<&StatusMessage>) {
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
            crate::model::StatusLevel::Info => theme::INFO,
            crate::model::StatusLevel::Success => theme::SUCCESS,
            crate::model::StatusLevel::Warning => theme::WARNING,
            crate::model::StatusLevel::Error => theme::ERROR,
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
            Span::styled(&cs.input.name, Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" to ", Style::default().fg(theme::TEXT)),
            Span::styled(commit.short_sha(), Style::default().fg(theme::SHA).add_modifier(Modifier::BOLD)),
            Span::styled("?", Style::default().fg(theme::TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled(msg_preview, Style::default().fg(theme::TEXT_DIM))),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(theme::SUCCESS)),
            Span::styled(" confirm  ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("n/q", Style::default().fg(theme::ERROR)),
            Span::styled(" cancel", Style::default().fg(theme::TEXT_DIM)),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::ACCENT))
                .style(Style::default().bg(theme::BG_DARK)),
        );

    frame.render_widget(dialog, dialog_area);
}
