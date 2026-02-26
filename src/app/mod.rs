//! Application core module
//!
//! This module contains the main application logic, including:
//! - `App`: The main application struct
//! - `state`: State types for different views
//! - `handler`: Input event handling

pub mod effects;
pub mod handler;
pub mod ports;
pub mod reducer;
pub mod state;
pub mod status;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::error::AppResult;
use crate::event::poll_key;
use crate::model::GitInput;
use crate::policy::build_lock_url;

use self::effects::{Effect, LockRequest};
use self::ports::{ClockPort, GitPort, NixPort, StatusCallback, SystemClock};
use self::reducer::{AppEvent, StatusCommand, Transition};
use crate::tui::Tui;
use crate::ui::render;

use self::state::EffectId;
pub use handler::Action;
pub use state::{AppState, ChangelogLoadedData, ChangelogState, ListState, TaskError, TaskResult};
use status::StatusMessage;

/// Main application struct
pub struct App {
    /// Path to the flake
    flake_path: PathBuf,
    /// Current state
    state: AppState,
    /// Nix port
    nix: Arc<dyn NixPort>,
    /// Git port
    git: Arc<dyn GitPort>,
    /// Cancellation token for async operations
    cancel_token: CancellationToken,
    /// Clock port for runtime time checks
    clock: Arc<dyn ClockPort>,
    /// Status message to display
    status_message: Option<StatusMessage>,
    /// Tick count for animations
    tick_count: u64,
    /// Channel for receiving task results
    task_rx: mpsc::UnboundedReceiver<TaskResult>,
    /// Channel for sending task results
    task_tx: mpsc::UnboundedSender<TaskResult>,
    /// Monotonic effect id counter
    next_effect_id: EffectId,
}

impl App {
    pub fn new_with_ports(
        flake_path: PathBuf,
        nix: Arc<dyn NixPort>,
        git: Arc<dyn GitPort>,
        cancel_token: CancellationToken,
    ) -> Self {
        let clock: Arc<dyn ClockPort> = Arc::new(SystemClock);
        Self::new_with_ports_and_clock(flake_path, nix, git, clock, cancel_token)
    }

    pub fn new_with_ports_and_clock(
        flake_path: PathBuf,
        nix: Arc<dyn NixPort>,
        git: Arc<dyn GitPort>,
        clock: Arc<dyn ClockPort>,
        cancel_token: CancellationToken,
    ) -> Self {
        let (task_tx, task_rx) = mpsc::unbounded_channel();
        Self {
            flake_path,
            state: AppState::Loading,
            nix,
            git,
            cancel_token,
            clock,
            status_message: None,
            tick_count: 0,
            task_rx,
            task_tx,
            next_effect_id: 1,
        }
    }

    pub async fn run(&mut self, tui: &mut Tui) -> AppResult<()> {
        self.dispatch_effects(vec![Effect::LoadFlake]);

        loop {
            if matches!(self.state, AppState::Quitting) {
                break;
            }

            if let Some(key) = poll_key(Duration::from_millis(16)) {
                self.handle_key(key);
            }

            if matches!(self.state, AppState::Quitting) {
                break;
            }

            tui.draw(|frame| self.render(frame))?;

            while let Ok(result) = self.task_rx.try_recv() {
                self.handle_task_result(result);
            }

            let transition = reducer::reduce(&mut self.state, AppEvent::Tick);
            self.apply_transition(transition);
            self.tick_count = self.tick_count.wrapping_add(1);

            if let Some(ref msg) = self.status_message {
                if msg.is_expired_at(self.clock.now()) {
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
                render::render_changelog(frame, cs.as_mut(), self.status_message.as_ref());
            }
            AppState::Quitting => {}
        }
    }

    /// Handle a key event
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        let action = handler::handle_key(&self.state, key);
        self.execute_action(action);
    }

    fn execute_action(&mut self, action: Action) {
        self.log_action(&action);
        let transition = reducer::reduce(&mut self.state, AppEvent::Action(&action));
        self.apply_transition(transition);
    }

    fn next_effect_id(&mut self) -> EffectId {
        let id = self.next_effect_id;
        self.next_effect_id = self.next_effect_id.wrapping_add(1);
        id
    }

    fn dispatch_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            let effect_id = self.next_effect_id();
            self.run_effect(effect_id, effect);
        }
    }

    fn run_effect(&self, effect_id: EffectId, effect: Effect) {
        match effect {
            Effect::LoadFlake => self.spawn_load_flake(effect_id),
            Effect::Update { path, names } => self.spawn_update(effect_id, path, names),
            Effect::UpdateAll { path } => self.spawn_update_all(effect_id, path),
            Effect::LoadChangelog { input, parent_list } => {
                self.spawn_load_changelog(effect_id, *input, *parent_list)
            }
            Effect::Lock(lock_request) => self.spawn_lock(effect_id, lock_request),
            Effect::CheckUpdates { inputs } => self.spawn_check_updates(effect_id, inputs),
        }
    }

    fn handle_task_result(&mut self, result: TaskResult) {
        self.log_task_result(&result);
        let transition = reducer::reduce(&mut self.state, AppEvent::TaskResult(&result));
        self.apply_transition(transition);
    }

    fn apply_transition(&mut self, transition: Transition) {
        if transition.cancel_requested {
            self.cancel_token.cancel();
        }
        self.apply_status_command(transition.status);
        self.dispatch_effects(transition.effects);
    }

    fn apply_status_command(&mut self, status: StatusCommand) {
        match status {
            StatusCommand::Keep => {}
            StatusCommand::Clear => self.status_message = None,
            StatusCommand::Info(msg) => self.status_message = Some(StatusMessage::info(msg)),
            StatusCommand::Success(msg) => {
                self.status_message = Some(StatusMessage::success_at(self.clock.now(), msg));
            }
            StatusCommand::Warning(msg) => {
                self.status_message = Some(StatusMessage::warning_at(self.clock.now(), msg));
            }
            StatusCommand::Error(msg) => {
                self.status_message = Some(StatusMessage::error_at(self.clock.now(), msg));
            }
        }
    }

    fn log_action(&self, action: &Action) {
        match action {
            Action::UpdateSelected(names) => {
                debug!(inputs = ?names, "Updating selected inputs");
            }
            Action::UpdateAll => {
                debug!("Updating all inputs");
            }
            Action::ConfirmLock => {
                debug!("Locking to commit");
            }
            _ => {}
        }
    }

    fn log_task_result(&self, result: &TaskResult) {
        match result {
            TaskResult::FlakeLoaded {
                effect_id,
                result: Ok(_),
            } => {
                debug!(effect_id = *effect_id, "flake loaded");
            }
            TaskResult::FlakeLoaded {
                effect_id,
                result: Err(error),
            } => {
                debug!(effect_id = *effect_id, "flake load failed");
                warn!(effect_id = *effect_id, error = %error, "Failed to load flake");
            }
            TaskResult::UpdateComplete {
                effect_id,
                result: Ok(()),
            } => {
                debug!(effect_id = *effect_id, "update complete");
            }
            TaskResult::UpdateComplete {
                effect_id,
                result: Err(error),
            } => {
                debug!(effect_id = *effect_id, "update failed");
                warn!(effect_id = *effect_id, error = %error, "Update failed");
            }
            TaskResult::ChangelogLoaded { effect_id, result } => {
                if result.is_ok() {
                    debug!(effect_id = *effect_id, "changelog loaded");
                } else if let Err(error) = result.as_ref() {
                    debug!(effect_id = *effect_id, "changelog load failed");
                    warn!(effect_id = *effect_id, error = %error, "Failed to load changelog");
                }
            }
            TaskResult::LockComplete {
                effect_id,
                result: Ok(()),
            } => {
                debug!(effect_id = *effect_id, "lock complete");
            }
            TaskResult::LockComplete {
                effect_id,
                result: Err(error),
            } => {
                debug!(effect_id = *effect_id, "lock failed");
                warn!(effect_id = *effect_id, error = %error, "Lock failed");
            }
            TaskResult::InputStatus {
                effect_id,
                name,
                status,
            } => {
                debug!(effect_id = *effect_id, input = %name, status = ?status, "input status update");
            }
        }
    }

    fn spawn_load_flake(&self, effect_id: EffectId) {
        let nix = Arc::clone(&self.nix);
        let path = self.flake_path.clone();
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix
                .load_metadata(&path)
                .await
                .map_err(|e| TaskError::from_message(e.to_string()));
            let _ = tx.send(TaskResult::FlakeLoaded { effect_id, result });
        });
    }

    fn spawn_update(&self, effect_id: EffectId, path: PathBuf, names: Vec<String>) {
        let nix = Arc::clone(&self.nix);
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix
                .update_inputs(&path, &names)
                .await
                .map_err(|e| TaskError::from_message(e.to_string()));
            let _ = tx.send(TaskResult::UpdateComplete { effect_id, result });
        });
    }

    fn spawn_update_all(&self, effect_id: EffectId, path: PathBuf) {
        let nix = Arc::clone(&self.nix);
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = nix
                .update_all(&path)
                .await
                .map_err(|e| TaskError::from_message(e.to_string()));
            let _ = tx.send(TaskResult::UpdateComplete { effect_id, result });
        });
    }

    fn spawn_load_changelog(&self, effect_id: EffectId, input: GitInput, parent_list: ListState) {
        let git = Arc::clone(&self.git);
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let result = git
                .get_changelog(&input)
                .await
                .map_err(|e| TaskError::from_message(e.to_string()));
            let _ = tx.send(TaskResult::ChangelogLoaded {
                effect_id,
                result: Box::new(result.map(|data| ChangelogLoadedData {
                    input,
                    data,
                    parent_list,
                })),
            });
        });
    }

    fn spawn_lock(&self, effect_id: EffectId, lock_request: LockRequest) {
        let nix = Arc::clone(&self.nix);
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let Some(lock_url) = build_lock_url(
                lock_request.forge_type,
                &lock_request.owner,
                &lock_request.repo,
                &lock_request.rev,
                lock_request.host.as_deref(),
            ) else {
                let result = Err(TaskError::external(
                    "Cannot generate lock URL for this input".to_string(),
                ));
                let _ = tx.send(TaskResult::LockComplete { effect_id, result });
                return;
            };

            let result = nix
                .lock_input(&lock_request.path, &lock_request.name, &lock_url)
                .await
                .map_err(|e| TaskError::from_message(e.to_string()));
            let _ = tx.send(TaskResult::LockComplete { effect_id, result });
        });
    }

    fn spawn_check_updates(&self, effect_id: EffectId, inputs: Vec<GitInput>) {
        let git = Arc::clone(&self.git);
        let tx = self.task_tx.clone();

        tokio::spawn(async move {
            let callback: StatusCallback<'_> = Box::new(move |name, status| {
                let _ = tx.send(TaskResult::InputStatus {
                    effect_id,
                    name: name.to_string(),
                    status,
                });
            });
            let _ = git.check_updates(&inputs, callback).await;
        });
    }
}
