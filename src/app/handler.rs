//! Input event handlers
//!
//! This module contains the input handling logic for different application states.

use crossterm::event::{KeyCode, KeyEvent};

use crate::event::KeyEventExt;
use crate::model::FlakeInput;

use super::state::{AppState, ChangelogState, ListState, StateKind};

/// Actions that can result from handling input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// No action needed
    None,
    /// Quit the application
    Quit,
    /// Cancel current operation and quit
    CancelAndQuit,
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
    ConfirmLock {
        input_name: String,
        lock_url: String,
    },
    /// Show warning message
    ShowWarning(String),
}

/// Handle key events based on current state
pub fn handle_key(state: &mut AppState, key: KeyEvent) -> Action {
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
fn handle_list_key(list: &mut ListState, key: KeyEvent) -> Action {
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
                list.clear_selection();
                Action::None
            } else {
                Action::Quit
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            list.cursor_down();
            Action::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            list.cursor_up();
            Action::None
        }
        KeyCode::Char(' ') => {
            if !is_busy {
                list.toggle_selection();
            }
            Action::None
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
                list.busy = true;
                Action::UpdateSelected(names)
            } else {
                Action::ShowWarning("No inputs selected".to_string())
            }
        }
        KeyCode::Char('U') => {
            if is_busy {
                return Action::None;
            }
            list.busy = true;
            Action::UpdateAll
        }
        KeyCode::Char('r') => {
            if is_busy {
                return Action::None;
            }
            list.busy = true;
            Action::Refresh
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
fn handle_changelog_key(cs: &mut ChangelogState, key: KeyEvent) -> Action {
    // Check if we're in confirm dialog
    if cs.is_confirming() {
        return handle_confirm_key(cs, key);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::CloseChangelog,
        KeyCode::Char('j') | KeyCode::Down => {
            cs.cursor_down();
            Action::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            cs.cursor_up();
            Action::None
        }
        KeyCode::Char(' ') => {
            cs.show_confirm();
            Action::None
        }
        _ => Action::None,
    }
}

/// Handle key events in confirm dialog
fn handle_confirm_key(cs: &mut ChangelogState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') => {
            let commit_idx = match cs.confirm_lock {
                Some(idx) => idx,
                None => return Action::None,
            };
            let commit = match cs.data.commits.get(commit_idx) {
                Some(c) => c,
                None => return Action::None,
            };

            let lock_url = cs.input.forge_type.lock_url(
                &cs.input.owner,
                &cs.input.repo,
                &commit.sha,
                cs.input.host.as_deref(),
            );

            if lock_url.is_empty() {
                cs.hide_confirm();
                return Action::ShowWarning("Cannot generate lock URL for this input".to_string());
            }

            Action::ConfirmLock {
                input_name: cs.input.name.clone(),
                lock_url,
            }
        }
        KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
            cs.hide_confirm();
            Action::None
        }
        _ => Action::None,
    }
}
