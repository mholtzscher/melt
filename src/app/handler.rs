//! Input event handlers
//!
//! This module contains the input handling logic for different application states.

use crossterm::event::{KeyCode, KeyEvent};

use crate::event::KeyEventExt;
use crate::model::{FlakeInput, InputName, LockUrl};

use super::state::{AppState, ChangelogState, ListMode, ListState, StateKind};

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
    UpdateSelected(Vec<InputName>),
    /// Update all inputs
    UpdateAll,
    /// Refresh flake data
    Refresh,
    /// Open commit history for a validated git input
    OpenChangelog { input: crate::model::GitInput },
    /// Close commit history and return to list
    CloseChangelog,
    /// Confirm lock to commit
    ConfirmLock {
        input_name: InputName,
        lock_url: LockUrl,
    },
    /// Show warning message
    ShowWarning(String),
}

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
                handle_changelog_key(cs.as_mut(), key)
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
    let is_busy = list.mode.is_busy();

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
            let names: Vec<InputName> = list
                .selected
                .iter()
                .filter(|name| {
                    list.flake
                        .inputs
                        .iter()
                        .any(|input| input.name() == name.as_str())
                })
                .cloned()
                .collect();

            if !names.is_empty() {
                list.mode = ListMode::UpdatingSelected {
                    inputs: names.clone(),
                };
                Action::UpdateSelected(names)
            } else {
                Action::ShowWarning("No inputs selected".to_string())
            }
        }
        KeyCode::Char('U') => {
            if is_busy {
                return Action::None;
            }
            list.mode = ListMode::UpdatingAll;
            Action::UpdateAll
        }
        KeyCode::Char('r') => {
            if is_busy {
                return Action::None;
            }
            list.mode = ListMode::Refreshing;
            Action::Refresh
        }
        KeyCode::Char('c') => {
            if is_busy {
                return Action::None;
            }
            let Some(idx) = list.current_index() else {
                return Action::None;
            };
            if let Some(FlakeInput::Git(input)) = list.flake.inputs.get(idx) {
                Action::OpenChangelog {
                    input: input.clone(),
                }
            } else {
                Action::ShowWarning("Commit history only available for git inputs".to_string())
            }
        }
        _ => Action::None,
    }
}

/// Handle key events in commit history view
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
            let Some(target) = cs.lock_target() else {
                return Action::None;
            };
            let Ok(lock_url) = cs.input.lock_url(target.target_rev()) else {
                cs.hide_confirm();
                return Action::ShowWarning("Cannot generate lock URL for this input".to_string());
            };

            Action::ConfirmLock {
                input_name: cs.input.input_name().clone(),
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
