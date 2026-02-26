use crate::model::FlakeInput;

use super::effects::Effect;
use super::state::TaskResult;
use super::{Action, AppState};

pub fn effects_for_action(state: &AppState, action: &Action) -> Vec<Effect> {
    match action {
        Action::None
        | Action::Quit
        | Action::CancelAndQuit
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
        Action::ConfirmLock {
            input_name,
            lock_url,
        } => match state {
            AppState::Changelog(cs) => vec![Effect::Lock {
                path: cs.parent_list.flake.path.clone(),
                name: input_name.clone(),
                lock_url: lock_url.clone(),
            }],
            _ => Vec::new(),
        },
    }
}

pub fn effects_for_task_result(result: &TaskResult) -> Vec<Effect> {
    match result {
        TaskResult::FlakeLoaded(Ok(flake)) => vec![Effect::CheckUpdates {
            inputs: flake.inputs.clone(),
        }],
        TaskResult::UpdateComplete(Ok(())) | TaskResult::LockComplete(Ok(())) => {
            vec![Effect::LoadFlake]
        }
        _ => Vec::new(),
    }
}
