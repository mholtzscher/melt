use std::mem;

use crate::model::{FlakeInput, UpdateStatus};

use super::effects::{Effect, LockRequest};
use super::state::TaskResult;
use super::{Action, AppState, ChangelogState, ListState};

pub enum AppEvent<'a> {
    Action(&'a Action),
    TaskResult(&'a TaskResult),
    Tick,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum StatusCommand {
    #[default]
    Keep,
    Clear,
    Info(String),
    Success(String),
    Warning(String),
    Error(String),
}

#[derive(Debug, Default)]
pub struct Transition {
    pub effects: Vec<Effect>,
    pub status: StatusCommand,
    pub cancel_requested: bool,
}

impl Transition {
    fn with_effects(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            ..Self::default()
        }
    }
}

pub fn reduce(state: &mut AppState, event: AppEvent<'_>) -> Transition {
    match event {
        AppEvent::Action(action) => reduce_action(state, action),
        AppEvent::TaskResult(result) => reduce_task_result(state, result),
        AppEvent::Tick => Transition::default(),
    }
}

pub fn effects_for_action(state: &AppState, action: &Action) -> Vec<Effect> {
    match action {
        Action::None
        | Action::Quit
        | Action::CancelAndQuit
        | Action::ListCursorDown
        | Action::ListCursorUp
        | Action::ListToggleSelection
        | Action::ListClearSelection
        | Action::ChangelogCursorDown
        | Action::ChangelogCursorUp
        | Action::ChangelogShowConfirm
        | Action::ChangelogHideConfirm
        | Action::CloseChangelog
        | Action::ShowWarning(_) => Vec::new(),
        Action::UpdateSelected(names) => match state {
            AppState::List(list) => vec![Effect::Update {
                path: list.flake.path.clone(),
                names: names.clone(),
            }],
            _ => Vec::new(),
        },
        Action::UpdateAll => match state {
            AppState::List(list) => vec![Effect::UpdateAll {
                path: list.flake.path.clone(),
            }],
            _ => Vec::new(),
        },
        Action::Refresh => vec![Effect::LoadFlake],
        Action::OpenChangelog { input_idx } => match state {
            AppState::List(list) => match list.flake.inputs.get(*input_idx) {
                Some(FlakeInput::Git(git_input)) => {
                    let input = git_input.clone();
                    let mut parent_list = list.clone();
                    parent_list.busy = false;
                    vec![Effect::LoadChangelog {
                        input: Box::new(input),
                        parent_list: Box::new(parent_list),
                    }]
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        },
        Action::ConfirmLock => match state {
            AppState::Changelog(cs) => {
                let Some(commit_idx) = cs.confirm_lock else {
                    return Vec::new();
                };
                let Some(commit) = cs.data.commits.get(commit_idx) else {
                    return Vec::new();
                };
                vec![Effect::Lock(LockRequest {
                    path: cs.parent_list.flake.path.clone(),
                    name: cs.input.name.clone(),
                    owner: cs.input.owner.clone(),
                    repo: cs.input.repo.clone(),
                    rev: commit.sha.clone(),
                    forge_type: cs.input.forge_type,
                    host: cs.input.host.clone(),
                })]
            }
            _ => Vec::new(),
        },
    }
}

pub fn effects_for_task_result(result: &TaskResult) -> Vec<Effect> {
    match result {
        TaskResult::FlakeLoaded {
            result: Ok(flake), ..
        } => vec![Effect::CheckUpdates {
            inputs: flake.inputs.clone(),
        }],
        TaskResult::UpdateComplete { result: Ok(()), .. }
        | TaskResult::LockComplete { result: Ok(()), .. } => vec![Effect::LoadFlake],
        _ => Vec::new(),
    }
}

fn reduce_action(state: &mut AppState, action: &Action) -> Transition {
    let mut transition = Transition::with_effects(effects_for_action(state, action));

    match action {
        Action::None => {}
        Action::ListCursorDown => {
            if let AppState::List(list) = state {
                list.cursor_down();
            }
        }
        Action::ListCursorUp => {
            if let AppState::List(list) = state {
                list.cursor_up();
            }
        }
        Action::ListToggleSelection => {
            if let AppState::List(list) = state {
                list.toggle_selection();
            }
        }
        Action::ListClearSelection => {
            if let AppState::List(list) = state {
                list.clear_selection();
            }
        }
        Action::ChangelogCursorDown => {
            if let AppState::Changelog(cs) = state {
                cs.cursor_down();
            }
        }
        Action::ChangelogCursorUp => {
            if let AppState::Changelog(cs) = state {
                cs.cursor_up();
            }
        }
        Action::ChangelogShowConfirm => {
            if let AppState::Changelog(cs) = state {
                cs.show_confirm();
            }
        }
        Action::ChangelogHideConfirm => {
            if let AppState::Changelog(cs) = state {
                cs.hide_confirm();
            }
        }
        Action::Quit | Action::CancelAndQuit => {
            *state = AppState::Quitting;
            transition.cancel_requested = true;
        }
        Action::UpdateSelected(names) => {
            transition.status =
                StatusCommand::Info(format!("Updating {} input(s)...", names.len()));
            if let AppState::List(list) = state {
                list.busy = true;
                for name in names {
                    list.update_statuses
                        .insert(name.clone(), UpdateStatus::Updating);
                }
            }
        }
        Action::UpdateAll => {
            transition.status = StatusCommand::Info("Updating all inputs...".to_string());
            if let AppState::List(list) = state {
                list.busy = true;
                for input in &list.flake.inputs {
                    list.update_statuses
                        .insert(input.name().to_string(), UpdateStatus::Updating);
                }
            }
        }
        Action::Refresh => {
            transition.status = StatusCommand::Info("Refreshing...".to_string());
            if let AppState::List(list) = state {
                list.busy = true;
            }
        }
        Action::OpenChangelog { .. } => {
            if let Some(Effect::LoadChangelog { parent_list, .. }) = transition.effects.first() {
                *state = AppState::LoadingChangelog(parent_list.as_ref().clone());
                transition.status = StatusCommand::Info("Loading changelog...".to_string());
            }
        }
        Action::CloseChangelog => close_changelog(state),
        Action::ConfirmLock => {
            if let AppState::Changelog(cs) = state {
                if let Some(commit_idx) = cs.confirm_lock {
                    if let Some(commit) = cs.data.commits.get(commit_idx) {
                        let short_sha = &commit.sha[..7.min(commit.sha.len())];
                        transition.status = StatusCommand::Info(format!(
                            "Locking {} to {}...",
                            cs.input.name, short_sha
                        ));
                    }
                }
            }
        }
        Action::ShowWarning(msg) => {
            transition.status = StatusCommand::Warning(msg.clone());
            if let AppState::Changelog(cs) = state {
                if cs.is_confirming() {
                    cs.hide_confirm();
                }
            }
        }
    }

    transition
}

fn reduce_task_result(state: &mut AppState, result: &TaskResult) -> Transition {
    let mut transition = Transition::with_effects(effects_for_task_result(result));

    match result {
        TaskResult::FlakeLoaded {
            effect_id: _,
            result: Ok(flake),
        } => {
            if let AppState::List(list) = state {
                list.update_flake(flake.clone());
            } else {
                *state = AppState::List(ListState::new(flake.clone()));
            }
            transition.status = StatusCommand::Clear;
        }
        TaskResult::FlakeLoaded {
            effect_id: _,
            result: Err(error),
        } => {
            *state = AppState::Error(format!("Failed to load flake: {}", error));
        }
        TaskResult::UpdateComplete {
            effect_id: _,
            result: Ok(()),
        } => {
            transition.status = StatusCommand::Success("Update complete".to_string());
            if let AppState::List(list) = state {
                list.clear_selection();
                list.update_statuses
                    .retain(|_, status| !matches!(status, UpdateStatus::Updating));
            }
        }
        TaskResult::UpdateComplete {
            effect_id: _,
            result: Err(error),
        } => {
            transition.status = StatusCommand::Error(format!("Update failed: {}", error));
            if let AppState::List(list) = state {
                list.busy = false;
                list.update_statuses
                    .retain(|_, status| !matches!(status, UpdateStatus::Updating));
            }
        }
        TaskResult::ChangelogLoaded {
            effect_id: _,
            result,
        } => match result.as_ref() {
            Ok(data) => {
                *state = AppState::Changelog(Box::new(ChangelogState::new(
                    data.input.clone(),
                    data.data.clone(),
                    data.parent_list.clone(),
                )));
                transition.status = StatusCommand::Clear;
            }
            Err(error) => {
                transition.status =
                    StatusCommand::Error(format!("Failed to load changelog: {}", error));
                if let AppState::LoadingChangelog(list) = mem::replace(state, AppState::Loading) {
                    *state = AppState::List(list);
                }
            }
        },
        TaskResult::LockComplete {
            effect_id: _,
            result: Ok(()),
        } => {
            transition.status = StatusCommand::Success("Locked successfully".to_string());
            if let AppState::Changelog(cs) = mem::replace(state, AppState::Loading) {
                let mut list = cs.parent_list;
                list.busy = true;
                *state = AppState::List(list);
            }
        }
        TaskResult::LockComplete {
            effect_id: _,
            result: Err(error),
        } => {
            transition.status = StatusCommand::Error(format!("Lock failed: {}", error));
            if let AppState::Changelog(cs) = state {
                cs.hide_confirm();
            }
        }
        TaskResult::InputStatus {
            effect_id: _,
            name,
            status,
        } => {
            if let AppState::List(list) = state {
                list.update_statuses.insert(name.clone(), status.clone());
            }
        }
    }

    transition
}

fn close_changelog(state: &mut AppState) {
    if let AppState::Changelog(cs) = mem::replace(state, AppState::Loading) {
        *state = AppState::List(cs.parent_list);
    }
}
