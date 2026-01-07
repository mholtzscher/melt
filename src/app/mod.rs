//! Application core module
//!
//! This module contains the main application logic, including:
//! - `App`: The main application struct
//! - `state`: State types for different views
//! - `handler`: Input event handling

pub mod handler;
pub mod state;

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::error::AppResult;
use crate::event::poll_key;
use crate::model::{FlakeInput, GitInput, StatusMessage};
use crate::service::{GitService, NixService};
use crate::tui::Tui;
use crate::ui::render;

pub use handler::Action;
pub use state::{AppState, ChangelogLoadedData, ChangelogState, ListState, TaskResult};

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

    /// Render the application UI
    fn render(&mut self, frame: &mut ratatui::Frame) {
        match &mut self.state {
            AppState::Loading => {
                render::render_loading(frame, "Loading flake...", self.tick_count);
            }
            AppState::Error(msg) => {
                render::render_error(frame, msg);
            }
            AppState::List(list) => {
                render::render_list(frame, list, self.status_message.as_ref(), self.tick_count);
            }
            AppState::LoadingChangelog(list) => {
                render::render_list(frame, list, self.status_message.as_ref(), self.tick_count);
            }
            AppState::Changelog(cs) => {
                render::render_changelog(frame, cs, self.status_message.as_ref());
            }
            AppState::Quitting => {}
        }
    }

    /// Handle a key event
    async fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        let action = handler::handle_key(&mut self.state, key);
        self.execute_action(action).await;
    }

    /// Execute an action returned from input handling
    async fn execute_action(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::Quit => {
                self.state = AppState::Quitting;
            }
            Action::CancelAndQuit => {
                self.cancel_token.cancel();
                self.state = AppState::Quitting;
            }
            Action::UpdateSelected(names) => {
                let count = names.len();
                self.status_message = Some(StatusMessage::info(format!(
                    "Updating {} input(s)...",
                    count
                )));
                if let AppState::List(list) = &self.state {
                    self.spawn_update(list.flake.path.clone(), names);
                }
            }
            Action::UpdateAll => {
                self.status_message = Some(StatusMessage::info("Updating all inputs..."));
                if let AppState::List(list) = &self.state {
                    self.spawn_update_all(list.flake.path.clone());
                }
            }
            Action::Refresh => {
                self.status_message = Some(StatusMessage::info("Refreshing..."));
                self.spawn_load_flake();
            }
            Action::OpenChangelog { input_idx } => {
                if let AppState::List(list) = &self.state {
                    if let Some(FlakeInput::Git(git_input)) = list.flake.inputs.get(input_idx) {
                        let input = git_input.clone();
                        let mut parent = list.clone();
                        parent.busy = false;
                        self.status_message = Some(StatusMessage::info("Loading changelog..."));
                        self.state = AppState::LoadingChangelog(parent.clone());
                        self.spawn_load_changelog(input, parent);
                    }
                }
            }
            Action::CloseChangelog => {
                self.close_changelog();
            }
            Action::ConfirmLock {
                input_name,
                lock_url,
            } => {
                if let AppState::Changelog(cs) = &self.state {
                    let commit_idx = cs.confirm_lock.unwrap_or(0);
                    if let Some(commit) = cs.data.commits.get(commit_idx) {
                        let short_sha = &commit.sha[..7.min(commit.sha.len())];
                        self.status_message = Some(StatusMessage::info(format!(
                            "Locking {} to {}...",
                            input_name, short_sha
                        )));
                    }
                    self.spawn_lock(cs.parent_list.flake.path.clone(), input_name, lock_url);
                }
            }
            Action::ShowWarning(msg) => {
                self.status_message = Some(StatusMessage::warning(msg));
            }
        }
    }

    /// Handle a result from a background task
    fn handle_task_result(&mut self, result: TaskResult) {
        match result {
            TaskResult::FlakeLoaded(Ok(flake)) => {
                let inputs = flake.inputs.clone();

                // Check if we're refreshing (already in List state) or initial load
                if let AppState::List(list) = &mut self.state {
                    list.update_flake(flake);
                } else {
                    self.state = AppState::List(ListState::new(flake));
                }

                // Clear any status message
                self.status_message = None;
                // Start checking for updates in background
                self.spawn_check_updates(inputs);
            }
            TaskResult::FlakeLoaded(Err(e)) => {
                self.state = AppState::Error(format!("Failed to load flake: {}", e));
            }
            TaskResult::UpdateComplete(Ok(())) => {
                self.status_message = Some(StatusMessage::success("Update complete"));
                // Clear selection and reload
                if let AppState::List(list) = &mut self.state {
                    list.clear_selection();
                }
                self.spawn_load_flake();
            }
            TaskResult::UpdateComplete(Err(e)) => {
                self.status_message = Some(StatusMessage::error(format!("Update failed: {}", e)));
                if let AppState::List(list) = &mut self.state {
                    list.busy = false;
                }
            }
            TaskResult::ChangelogLoaded(Ok(data)) => {
                self.state = AppState::Changelog(ChangelogState::new(
                    data.input,
                    data.data,
                    data.parent_list,
                ));
                self.status_message = None;
            }
            TaskResult::ChangelogLoaded(Err(e)) => {
                self.status_message = Some(StatusMessage::error(format!(
                    "Failed to load changelog: {}",
                    e
                )));
                // Return to list from loading changelog state
                if let AppState::LoadingChangelog(list) =
                    std::mem::replace(&mut self.state, AppState::Loading)
                {
                    self.state = AppState::List(list);
                }
            }
            TaskResult::LockComplete(Ok(())) => {
                self.status_message = Some(StatusMessage::success("Locked successfully"));
                // Return to list and reload
                if let AppState::Changelog(cs) =
                    std::mem::replace(&mut self.state, AppState::Loading)
                {
                    let mut list = cs.parent_list;
                    list.busy = true;
                    self.state = AppState::List(list);
                }
                self.spawn_load_flake();
            }
            TaskResult::LockComplete(Err(e)) => {
                self.status_message = Some(StatusMessage::error(format!("Lock failed: {}", e)));
                if let AppState::Changelog(cs) = &mut self.state {
                    cs.hide_confirm();
                }
            }
            TaskResult::InputStatus { name, status } => {
                if let AppState::List(list) = &mut self.state {
                    list.update_statuses.insert(name, status);
                }
            }
        }
    }

    /// Spawn a background task to load flake metadata
    fn spawn_load_flake(&self) {
        let nix = self.nix.clone();
        let path = self.flake_path.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.load_metadata(&path).await;
            let _ = tx.send(TaskResult::FlakeLoaded(result));
        });
    }

    /// Spawn a background task to update inputs
    fn spawn_update(&self, path: PathBuf, names: Vec<String>) {
        let nix = self.nix.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.update_inputs(&path, &names).await;
            let _ = tx.send(TaskResult::UpdateComplete(result));
        });
    }

    /// Spawn a background task to update all inputs
    fn spawn_update_all(&self, path: PathBuf) {
        let nix = self.nix.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.update_all(&path).await;
            let _ = tx.send(TaskResult::UpdateComplete(result));
        });
    }

    /// Spawn a background task to load changelog
    fn spawn_load_changelog(&self, input: GitInput, parent_list: ListState) {
        let git = self.git.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = git.get_changelog(&input).await;
            let _ = tx.send(TaskResult::ChangelogLoaded(result.map(|data| {
                ChangelogLoadedData {
                    input,
                    data,
                    parent_list,
                }
            })));
        });
    }

    /// Spawn a background task to lock an input
    fn spawn_lock(&self, path: PathBuf, name: String, lock_url: String) {
        let nix = self.nix.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix.lock_input(&path, &name, &lock_url).await;
            let _ = tx.send(TaskResult::LockComplete(result));
        });
    }

    /// Spawn background tasks to check for updates on all inputs
    fn spawn_check_updates(&self, inputs: Vec<FlakeInput>) {
        let git = self.git.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let _ = git
                .check_updates(&inputs, |name, status| {
                    let _ = tx.send(TaskResult::InputStatus {
                        name: name.to_string(),
                        status,
                    });
                })
                .await;
        });
    }

    /// Close changelog and return to list
    fn close_changelog(&mut self) {
        if let AppState::Changelog(cs) = std::mem::replace(&mut self.state, AppState::Loading) {
            self.state = AppState::List(cs.parent_list);
        }
    }
}
