//! Integration tests for app-core boundaries.

use std::path::PathBuf;

use chrono::Utc;
use melt::app::effects::Effect;
use melt::app::reducer::effects_for_action;
use melt::app::{Action, AppState, ChangelogState, ListState};
use melt::model::{ChangelogData, Commit, FlakeData, FlakeInput, ForgeType, GitInput, PathInput};

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
    let state = AppState::Changelog(Box::new(ChangelogState::new(input, changelog, parent_list)));

    let action = Action::ConfirmLock {
        input_name: "nixpkgs".to_string(),
        lock_url: "github:NixOS/nixpkgs/deadbeef".to_string(),
    };

    let effects = effects_for_action(&state, &action);
    assert_eq!(effects.len(), 1);

    match &effects[0] {
        Effect::Lock {
            path,
            name,
            lock_url,
        } => {
            assert_eq!(path, &PathBuf::from("/tmp/flake"));
            assert_eq!(name, "nixpkgs");
            assert_eq!(lock_url, "github:NixOS/nixpkgs/deadbeef");
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
fn list_state_cursor_stays_in_bounds() {
    let mut list = list_with_git_input("nixpkgs");

    list.cursor_up();
    assert_eq!(list.cursor, 0);

    list.cursor_down();
    assert_eq!(list.cursor, 0);
}

#[test]
fn list_state_selection_toggle_round_trip() {
    let mut list = list_with_git_input("nixpkgs");

    assert!(!list.has_selection());
    list.toggle_selection();
    assert!(list.has_selection());
    assert!(list.selected.contains(&0));

    list.toggle_selection();
    assert!(!list.has_selection());
}

#[test]
fn changelog_confirm_only_when_commits_exist() {
    let input = sample_git_input("nixpkgs");
    let parent_list = list_with_git_input("nixpkgs");

    let mut empty = ChangelogState::new(
        input.clone(),
        ChangelogData {
            commits: Vec::new(),
            locked_idx: None,
        },
        parent_list.clone(),
    );
    empty.show_confirm();
    assert!(empty.confirm_lock.is_none());

    let mut non_empty = ChangelogState::new(
        input,
        ChangelogData {
            commits: vec![sample_commit("deadbeef", false)],
            locked_idx: None,
        },
        parent_list,
    );
    non_empty.show_confirm();
    assert_eq!(non_empty.confirm_lock, Some(0));
}
