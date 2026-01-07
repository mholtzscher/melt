//! Application state types
//!
//! This module contains all the state types used by the application,
//! including the main AppState enum and view-specific states.

use std::collections::{HashMap, HashSet};

use ratatui::widgets::TableState;

use crate::error::{AppError, GitError};
use crate::model::{ChangelogData, FlakeData, GitInput, UpdateStatus};

/// Application state machine
#[derive(Debug)]
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

impl AppState {
    /// Get the kind of state for pattern matching without borrowing
    pub fn kind(&self) -> StateKind {
        match self {
            AppState::Loading => StateKind::Loading,
            AppState::Error(_) => StateKind::Error,
            AppState::List(_) => StateKind::List,
            AppState::Changelog(_) => StateKind::Changelog,
            AppState::LoadingChangelog(_) => StateKind::LoadingChangelog,
            AppState::Quitting => StateKind::Quitting,
        }
    }
}

/// Simple enum for state discrimination without borrowing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Loading,
    Error,
    List,
    Changelog,
    LoadingChangelog,
    Quitting,
}

/// State for the list view
#[derive(Debug)]
pub struct ListState {
    pub flake: FlakeData,
    pub cursor: usize,
    pub selected: HashSet<usize>,
    pub table_state: TableState,
    pub update_statuses: HashMap<String, UpdateStatus>,
    /// True when a background operation is in progress
    pub busy: bool,
}

impl ListState {
    /// Create a new ListState from flake data
    pub fn new(flake: FlakeData) -> Self {
        let mut table_state = TableState::default();
        if !flake.inputs.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            flake,
            cursor: 0,
            selected: HashSet::new(),
            table_state,
            update_statuses: HashMap::new(),
            busy: false,
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        if self.cursor < self.flake.inputs.len().saturating_sub(1) {
            self.cursor += 1;
            self.table_state.select(Some(self.cursor));
        }
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.table_state.select(Some(self.cursor));
        }
    }

    /// Toggle selection at cursor
    pub fn toggle_selection(&mut self) {
        if self.selected.contains(&self.cursor) {
            self.selected.remove(&self.cursor);
        } else {
            self.selected.insert(self.cursor);
        }
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Check if there are any selections
    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Get the number of inputs
    pub fn input_count(&self) -> usize {
        self.flake.inputs.len()
    }

    /// Update with new flake data (for refresh)
    pub fn update_flake(&mut self, flake: FlakeData) {
        self.flake = flake;
        self.busy = false;
        // Clamp cursor to new input count
        if self.cursor >= self.flake.inputs.len() {
            self.cursor = self.flake.inputs.len().saturating_sub(1);
            self.table_state.select(Some(self.cursor));
        }
        // Clear selections that are now out of bounds
        self.selected.retain(|&i| i < self.flake.inputs.len());
        // Clear old update statuses
        self.update_statuses.clear();
    }
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
#[derive(Debug)]
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

impl ChangelogState {
    /// Create a new ChangelogState
    pub fn new(
        input: GitInput,
        input_idx: usize,
        data: ChangelogData,
        parent_list: ListState,
    ) -> Self {
        let cursor = data.locked_idx.unwrap_or(0);
        let mut table_state = TableState::default();
        if !data.commits.is_empty() {
            table_state.select(Some(cursor));
        }
        Self {
            input,
            input_idx,
            data,
            cursor,
            table_state,
            confirm_lock: None,
            parent_list,
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        if self.cursor < self.data.commits.len().saturating_sub(1) {
            self.cursor += 1;
            self.table_state.select(Some(self.cursor));
        }
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.table_state.select(Some(self.cursor));
        }
    }

    /// Show confirm dialog for current cursor position
    pub fn show_confirm(&mut self) {
        if !self.data.commits.is_empty() {
            self.confirm_lock = Some(self.cursor);
        }
    }

    /// Hide confirm dialog
    pub fn hide_confirm(&mut self) {
        self.confirm_lock = None;
    }

    /// Check if confirm dialog is showing
    pub fn is_confirming(&self) -> bool {
        self.confirm_lock.is_some()
    }
}

/// Data returned when changelog is loaded
#[derive(Debug)]
pub struct ChangelogLoadedData {
    pub input: GitInput,
    pub input_idx: usize,
    pub data: ChangelogData,
    pub parent_list: ListState,
}

/// Messages from background tasks
#[derive(Debug)]
pub enum TaskResult {
    /// Flake metadata loaded
    FlakeLoaded(Result<FlakeData, AppError>),
    /// Input update completed
    UpdateComplete(Result<(), AppError>),
    /// Changelog loaded
    ChangelogLoaded(Result<ChangelogLoadedData, GitError>),
    /// Lock completed
    LockComplete(Result<(), AppError>),
    /// Status update for a single input
    InputStatus { name: String, status: UpdateStatus },
}
