use std::path::PathBuf;

use chrono::Utc;
use melt::app::effects::Effect;
use melt::app::reducer::{
    effects_for_action, effects_for_task_result, reduce, AppEvent, StatusCommand,
};
use melt::app::{Action, AppState, ChangelogState, ListState, TaskError, TaskResult};
use melt::model::{
    ChangelogData, Commit, FlakeData, FlakeInput, ForgeType, GitInput, PathInput, UpdateStatus,
};

fn sample_git_input(name: &str) -> GitInput {
    GitInput {
        name: name.to_string(),
        owner: "NixOS".to_string(),
        repo: "nixpkgs".to_string(),
        forge_type: ForgeType::GitHub,
        host: None,
        reference: Some("nixos-unstable".to_string()),
        rev: "abc1234def5678".to_string(),
        last_modified: 1_700_000_000,
        url: "github:NixOS/nixpkgs".to_string(),
    }
}

fn sample_commit(sha: &str, is_locked: bool) -> Commit {
    Commit {
        sha: sha.to_string(),
        message: "sample message".to_string(),
        author: "sample-author".to_string(),
        date: Utc::now(),
        is_locked,
    }
}

fn sample_flake(inputs: Vec<FlakeInput>) -> FlakeData {
    FlakeData {
        path: PathBuf::from("/tmp/flake"),
        inputs,
    }
}

fn list_with_git_input(name: &str) -> ListState {
    let flake = sample_flake(vec![FlakeInput::Git(sample_git_input(name))]);
    ListState::new(flake)
}

#[test]
fn refresh_plans_load_flake_effect() {
    let state = AppState::Loading;
    let effects = effects_for_action(&state, &Action::Refresh);

    assert!(matches!(effects.as_slice(), [Effect::LoadFlake]));
}

#[test]
fn update_selected_plans_update_effect_with_path_and_names() {
    let state = AppState::List(list_with_git_input("nixpkgs"));
    let action = Action::UpdateSelected(vec!["nixpkgs".to_string()]);

    let effects = effects_for_action(&state, &action);
    assert_eq!(effects.len(), 1);

    match &effects[0] {
        Effect::Update { path, names } => {
            assert_eq!(path, &PathBuf::from("/tmp/flake"));
            assert_eq!(names, &vec!["nixpkgs".to_string()]);
        }
        other => panic!("expected Update effect, got {:?}", other),
    }
}

#[test]
fn update_selected_outside_list_plans_nothing() {
    let state = AppState::Loading;
    let action = Action::UpdateSelected(vec!["nixpkgs".to_string()]);

    let effects = effects_for_action(&state, &action);
    assert!(effects.is_empty());
}

#[test]
fn open_changelog_for_git_input_plans_load_changelog_effect() {
    let mut list = list_with_git_input("nixpkgs");
    list.busy = true;
    let state = AppState::List(list);

    let effects = effects_for_action(&state, &Action::OpenChangelog { input_idx: 0 });
    assert_eq!(effects.len(), 1);

    match &effects[0] {
        Effect::LoadChangelog { input, parent_list } => {
            assert_eq!(input.name, "nixpkgs");
            assert!(!parent_list.busy, "parent list should be reset to not busy");
        }
        other => panic!("expected LoadChangelog effect, got {:?}", other),
    }
}

#[test]
fn open_changelog_for_non_git_input_plans_nothing() {
    let flake = sample_flake(vec![FlakeInput::Path(PathInput {
        name: "local".to_string(),
    })]);
    let state = AppState::List(ListState::new(flake));

    let effects = effects_for_action(&state, &Action::OpenChangelog { input_idx: 0 });
    assert!(effects.is_empty());
}

#[test]
fn confirm_lock_in_changelog_state_plans_lock_effect() {
    let input = sample_git_input("nixpkgs");
    let changelog = ChangelogData {
        commits: vec![sample_commit("deadbeef", false)],
        locked_idx: None,
    };
    let parent_list = list_with_git_input("nixpkgs");
    let mut cs = ChangelogState::new(input, changelog, parent_list);
    cs.show_confirm();
    let state = AppState::Changelog(Box::new(cs));

    let action = Action::ConfirmLock;

    let effects = effects_for_action(&state, &action);
    assert_eq!(effects.len(), 1);

    match &effects[0] {
        Effect::Lock(lock_request) => {
            assert_eq!(lock_request.path, PathBuf::from("/tmp/flake"));
            assert_eq!(lock_request.name, "nixpkgs");
            assert_eq!(lock_request.owner, "NixOS");
            assert_eq!(lock_request.repo, "nixpkgs");
            assert_eq!(lock_request.rev, "deadbeef");
            assert_eq!(lock_request.forge_type, ForgeType::GitHub);
            assert!(lock_request.host.is_none());
        }
        other => panic!("expected Lock effect, got {:?}", other),
    }
}

#[test]
fn quit_actions_plan_no_effects() {
    let state = AppState::Loading;

    assert!(effects_for_action(&state, &Action::Quit).is_empty());
    assert!(effects_for_action(&state, &Action::CancelAndQuit).is_empty());
}

#[test]
fn flake_loaded_task_result_plans_check_updates_effect() {
    let flake = sample_flake(vec![FlakeInput::Git(sample_git_input("nixpkgs"))]);
    let result = TaskResult::FlakeLoaded {
        effect_id: 42,
        result: Ok(flake),
    };

    let effects = effects_for_task_result(&result);
    assert_eq!(effects.len(), 1);

    match &effects[0] {
        Effect::CheckUpdates { inputs } => assert_eq!(inputs.len(), 1),
        other => panic!("expected CheckUpdates effect, got {:?}", other),
    }
}

#[test]
fn successful_update_task_result_plans_reload_effect() {
    let result = TaskResult::UpdateComplete {
        effect_id: 7,
        result: Ok(()),
    };

    let effects = effects_for_task_result(&result);
    assert!(matches!(effects.as_slice(), [Effect::LoadFlake]));
}

#[test]
fn failed_update_task_result_plans_no_follow_up_effects() {
    let result = TaskResult::UpdateComplete {
        effect_id: 7,
        result: Err(TaskError::external("boom")),
    };

    let effects = effects_for_task_result(&result);
    assert!(effects.is_empty());
}

#[test]
fn reducer_handles_action_task_result_and_tick_events() {
    let mut action_state = AppState::Loading;
    let refresh = Action::Refresh;
    let action_transition = reduce(&mut action_state, AppEvent::Action(&refresh));
    assert!(matches!(
        action_transition.effects.as_slice(),
        [Effect::LoadFlake]
    ));

    let mut task_state = AppState::Loading;
    let result = TaskResult::UpdateComplete {
        effect_id: 1,
        result: Ok(()),
    };
    let task_transition = reduce(&mut task_state, AppEvent::TaskResult(&result));
    assert!(matches!(
        task_transition.effects.as_slice(),
        [Effect::LoadFlake]
    ));

    let mut tick_state = AppState::Loading;
    let tick_transition = reduce(&mut tick_state, AppEvent::Tick);
    assert!(tick_transition.effects.is_empty());
    assert_eq!(tick_transition.status, StatusCommand::Keep);
}

#[test]
fn reducer_quit_sets_quitting_and_requests_cancel() {
    let mut state = AppState::Loading;
    let action = Action::Quit;

    let transition = reduce(&mut state, AppEvent::Action(&action));

    assert!(matches!(state, AppState::Quitting));
    assert!(transition.cancel_requested);
    assert!(transition.effects.is_empty());
    assert_eq!(transition.status, StatusCommand::Keep);
}

#[test]
fn reducer_open_changelog_moves_to_loading_state() {
    let mut state = AppState::List(list_with_git_input("nixpkgs"));
    let action = Action::OpenChangelog { input_idx: 0 };

    let transition = reduce(&mut state, AppEvent::Action(&action));

    assert!(matches!(state, AppState::LoadingChangelog(_)));
    assert!(matches!(
        transition.effects.as_slice(),
        [Effect::LoadChangelog { .. }]
    ));
    assert_eq!(
        transition.status,
        StatusCommand::Info("Loading changelog...".to_string())
    );
}

#[test]
fn reducer_update_complete_success_mutates_list_state() {
    let mut list = list_with_git_input("nixpkgs");
    list.selected.insert(0);
    list.update_statuses
        .insert("nixpkgs".to_string(), UpdateStatus::Updating);
    let mut state = AppState::List(list);

    let result = TaskResult::UpdateComplete {
        effect_id: 9,
        result: Ok(()),
    };

    let transition = reduce(&mut state, AppEvent::TaskResult(&result));

    assert!(matches!(transition.effects.as_slice(), [Effect::LoadFlake]));
    assert_eq!(
        transition.status,
        StatusCommand::Success("Update complete".to_string())
    );
    if let AppState::List(list) = state {
        assert!(list.selected.is_empty());
        assert!(list.update_statuses.is_empty());
    } else {
        panic!("expected list state after update complete");
    }
}
