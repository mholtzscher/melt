//! Input event handlers
//!
//! This module contains the input handling logic for different application states.

use crossterm::event::{KeyCode, KeyEvent};

use crate::event::KeyEventExt;
use crate::model::FlakeInput;

use super::state::{AppState, StateKind};

/// Actions that can result from handling input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// No action needed
    None,
    /// Quit the application
    Quit,
    /// Cancel current operation and quit
    CancelAndQuit,
    /// Move list cursor down
    ListCursorDown,
    /// Move list cursor up
    ListCursorUp,
    /// Toggle list selection at cursor
    ListToggleSelection,
    /// Clear all list selections
    ListClearSelection,
    /// Move changelog cursor down
    ChangelogCursorDown,
    /// Move changelog cursor up
    ChangelogCursorUp,
    /// Show changelog lock confirmation dialog
    ChangelogShowConfirm,
    /// Hide changelog lock confirmation dialog
    ChangelogHideConfirm,
    /// Update selected inputs
    UpdateSelected(Vec<String>),
    /// Update all inputs
    UpdateAll,
    /// Refresh flake data
    Refresh,
    /// Open changelog for input at index
    OpenChangelog { input_idx: usize },
    /// Close changelog and return to list
    CloseChangelog,
    /// Confirm lock to commit
    ConfirmLock,
    /// Show warning message
    ShowWarning(String),
}

pub fn handle_key(state: &AppState, key: KeyEvent) -> Action {
    match state.kind() {
        StateKind::Loading | StateKind::LoadingChangelog => {
            if key.is_quit() {
                Action::CancelAndQuit
            } else {
                Action::None
            }
        }
        StateKind::Error => Action::Quit,
        StateKind::List => {
            if let AppState::List(list) = state {
                handle_list_key(list, key)
            } else {
                Action::None
            }
        }
        StateKind::Changelog => {
            if let AppState::Changelog(cs) = state {
                handle_changelog_key(cs, key)
            } else {
                Action::None
            }
        }
        StateKind::Quitting => Action::None,
    }
}

/// Handle key events in list view
fn handle_list_key(list: &super::state::ListState, key: KeyEvent) -> Action {
    let input_count = list.input_count();
    let has_selection = list.has_selection();
    let is_busy = list.busy;

    if input_count == 0 {
        if key.is_quit() {
            return Action::Quit;
        }
        return Action::None;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if has_selection {
                Action::ListClearSelection
            } else {
                Action::Quit
            }
        }
        KeyCode::Char('j') | KeyCode::Down => Action::ListCursorDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ListCursorUp,
        KeyCode::Char(' ') => {
            if is_busy {
                Action::None
            } else {
                Action::ListToggleSelection
            }
        }
        KeyCode::Char('u') => {
            if is_busy {
                return Action::None;
            }
            let names: Vec<String> = list
                .selected
                .iter()
                .filter_map(|&i| list.flake.inputs.get(i))
                .map(|input| input.name().to_string())
                .collect();

            if !names.is_empty() {
                Action::UpdateSelected(names)
            } else {
                Action::ShowWarning("No inputs selected".to_string())
            }
        }
        KeyCode::Char('U') => {
            if is_busy {
                Action::None
            } else {
                Action::UpdateAll
            }
        }
        KeyCode::Char('r') => {
            if is_busy {
                Action::None
            } else {
                Action::Refresh
            }
        }
        KeyCode::Char('c') => {
            if is_busy {
                return Action::None;
            }
            let idx = list.cursor;
            if let Some(FlakeInput::Git(_)) = list.flake.inputs.get(idx) {
                Action::OpenChangelog { input_idx: idx }
            } else {
                Action::ShowWarning("Changelog only available for git inputs".to_string())
            }
        }
        _ => Action::None,
    }
}

/// Handle key events in changelog view
fn handle_changelog_key(cs: &super::state::ChangelogState, key: KeyEvent) -> Action {
    // Check if we're in confirm dialog
    if cs.is_confirming() {
        return handle_confirm_key(cs, key);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::CloseChangelog,
        KeyCode::Char('j') | KeyCode::Down => Action::ChangelogCursorDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ChangelogCursorUp,
        KeyCode::Char(' ') => Action::ChangelogShowConfirm,
        _ => Action::None,
    }
}

/// Handle key events in confirm dialog
fn handle_confirm_key(cs: &super::state::ChangelogState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') => {
            let commit_idx = match cs.confirm_lock {
                Some(idx) => idx,
                None => return Action::None,
            };
            if cs.data.commits.get(commit_idx).is_none() {
                return Action::None;
            }
            Action::ConfirmLock
        }
        KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => Action::ChangelogHideConfirm,
        _ => Action::None,
    }
}
