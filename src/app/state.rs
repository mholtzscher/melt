//! Application state types
//!
//! This module contains all the state types used by the application,
//! including the main AppState enum and view-specific states.

use std::collections::{HashMap, HashSet};

use ratatui::widgets::TableState;

use crate::error::{AppError, GitError};
use crate::model::{ChangelogData, FlakeData, GitInput, GitRev, InputName, UpdateStatus};

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
    Changelog(Box<ChangelogState>),
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

/// Cursor position in the list view. Constructed by `ListState` so it is
/// always valid for the current non-empty input list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListCursor {
    index: usize,
}

impl ListCursor {
    pub fn index(self) -> usize {
        self.index
    }

    fn new(index: usize, len: usize) -> Option<Self> {
        if len == 0 {
            None
        } else {
            Some(Self {
                index: index.min(len - 1),
            })
        }
    }
}

/// Current operation mode for the list view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMode {
    Idle,
    Refreshing,
    UpdatingAll,
    UpdatingSelected { inputs: Vec<InputName> },
}

impl ListMode {
    pub fn is_busy(&self) -> bool {
        !matches!(self, ListMode::Idle)
    }
}

/// State for the list view
#[derive(Debug)]
pub struct ListState {
    pub flake: FlakeData,
    pub cursor: Option<ListCursor>,
    pub selected: HashSet<InputName>,
    pub table_state: TableState,
    pub update_statuses: HashMap<InputName, UpdateStatus>,
    pub mode: ListMode,
}

impl ListState {
    /// Create a new ListState from flake data
    pub fn new(flake: FlakeData) -> Self {
        let mut table_state = TableState::default();
        if !flake.inputs.is_empty() {
            table_state.select(Some(0));
        }
        let cursor = ListCursor::new(0, flake.inputs.len());
        Self {
            flake,
            cursor,
            selected: HashSet::new(),
            table_state,
            update_statuses: HashMap::new(),
            mode: ListMode::Idle,
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let next = (cursor.index() + 1).min(self.flake.inputs.len().saturating_sub(1));
        self.cursor = ListCursor::new(next, self.flake.inputs.len());
        self.table_state.select(Some(next));
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let next = cursor.index().saturating_sub(1);
        self.cursor = ListCursor::new(next, self.flake.inputs.len());
        self.table_state.select(Some(next));
    }

    /// Toggle selection at cursor
    pub fn toggle_selection(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some(input) = self.flake.inputs.get(cursor.index()) else {
            return;
        };
        let Ok(name) = InputName::new(input.name()) else {
            return;
        };
        if self.selected.contains(&name) {
            self.selected.remove(&name);
        } else {
            self.selected.insert(name);
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

    /// Get the current cursor index, if the list is non-empty.
    pub fn current_index(&self) -> Option<usize> {
        self.cursor.map(ListCursor::index)
    }

    /// Update with new flake data (for refresh)
    pub fn update_flake(&mut self, flake: FlakeData) {
        self.flake = flake;
        self.mode = ListMode::Idle;
        // Clamp cursor to new input count, or clear it for an empty list.
        let next_cursor = self
            .cursor
            .and_then(|cursor| ListCursor::new(cursor.index(), self.flake.inputs.len()))
            .or_else(|| ListCursor::new(0, self.flake.inputs.len()));
        self.cursor = next_cursor;
        self.table_state.select(self.cursor.map(ListCursor::index));
        // Keep only selections whose input names still exist after refresh.
        let existing_names: HashSet<InputName> = self
            .flake
            .inputs
            .iter()
            .filter_map(|input| InputName::new(input.name()).ok())
            .collect();
        self.selected.retain(|name| existing_names.contains(name));
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
            mode: self.mode.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockTarget {
    commit_idx: usize,
    target_rev: GitRev,
}

impl LockTarget {
    pub fn new(commit_idx: usize, commits: &[crate::model::Commit]) -> Option<Self> {
        let commit = commits.get(commit_idx)?;
        let target_rev = GitRev::new(commit.sha.clone()).ok()?;
        Some(Self {
            commit_idx,
            target_rev,
        })
    }

    pub fn commit_idx(&self) -> usize {
        self.commit_idx
    }

    pub fn target_rev(&self) -> &GitRev {
        &self.target_rev
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangelogMode {
    Browsing,
    ConfirmingLock { target: LockTarget },
}

/// State for the changelog view
#[derive(Debug)]
pub struct ChangelogState {
    /// The input we're showing changelog for
    pub input: GitInput,
    /// The changelog data
    pub data: ChangelogData,
    /// Current cursor position
    pub cursor: usize,
    /// Table state for rendering
    pub table_state: TableState,
    pub mode: ChangelogMode,
    /// Parent list state (kept for returning)
    pub parent_list: ListState,
}

impl ChangelogState {
    /// Create a new ChangelogState
    pub fn new(input: GitInput, data: ChangelogData, parent_list: ListState) -> Self {
        let cursor = data.locked_index().unwrap_or(0);
        let mut table_state = TableState::default();
        if !data.commits.is_empty() {
            table_state.select(Some(cursor));
        }
        Self {
            input,
            data,
            cursor,
            table_state,
            mode: ChangelogMode::Browsing,
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
        if let Some(target) = LockTarget::new(self.cursor, &self.data.commits) {
            self.mode = ChangelogMode::ConfirmingLock { target };
        }
    }

    /// Hide confirm dialog
    pub fn hide_confirm(&mut self) {
        self.mode = ChangelogMode::Browsing;
    }

    /// Check if confirm dialog is showing
    pub fn is_confirming(&self) -> bool {
        matches!(self.mode, ChangelogMode::ConfirmingLock { .. })
    }

    pub fn lock_target(&self) -> Option<&LockTarget> {
        match &self.mode {
            ChangelogMode::ConfirmingLock { target } => Some(target),
            ChangelogMode::Browsing => None,
        }
    }
}

/// Data returned when changelog is loaded
#[derive(Debug)]
pub struct ChangelogLoadedData {
    pub input: GitInput,
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
    ChangelogLoaded(Box<Result<ChangelogLoadedData, GitError>>),
    /// Lock completed
    LockComplete(Result<(), AppError>),
    /// Status update for a single input
    InputStatus {
        name: InputName,
        status: UpdateStatus,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FlakeInput, PathInput};
    use std::path::PathBuf;

    fn flake(names: &[&str]) -> FlakeData {
        FlakeData {
            path: PathBuf::from("/tmp/flake"),
            inputs: names
                .iter()
                .map(|name| {
                    FlakeInput::Path(PathInput {
                        name: (*name).to_string(),
                    })
                })
                .collect(),
        }
    }

    #[test]
    fn list_state_empty_list_has_no_cursor() {
        let list = ListState::new(flake(&[]));
        assert_eq!(list.current_index(), None);
    }

    #[test]
    fn list_state_selection_survives_reorder_by_name() {
        let mut list = ListState::new(flake(&["a", "b"]));
        list.cursor_down();
        list.toggle_selection();
        assert!(list.selected.contains(&InputName::new("b").unwrap()));

        list.update_flake(flake(&["b", "a"]));

        assert!(list.selected.contains(&InputName::new("b").unwrap()));
        assert!(!list.selected.contains(&InputName::new("a").unwrap()));
    }

    #[test]
    fn list_state_selection_drops_missing_names_after_refresh() {
        let mut list = ListState::new(flake(&["a", "b"]));
        list.toggle_selection();
        list.cursor_down();
        list.toggle_selection();

        list.update_flake(flake(&["b"]));

        assert_eq!(list.selected.len(), 1);
        assert!(list.selected.contains(&InputName::new("b").unwrap()));
    }
}
