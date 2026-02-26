//! Integration-style reducer smoke coverage for full user flows.

use std::path::PathBuf;

use chrono::Utc;
use melt::app::effects::Effect;
use melt::app::reducer::{reduce, AppEvent, StatusCommand};
use melt::app::{Action, AppState, ListState, TaskResult};
use melt::model::{ChangelogData, Commit, FlakeData, FlakeInput, ForgeType, GitInput};

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

fn sample_commit(sha: &str) -> Commit {
    Commit {
        sha: sha.to_string(),
        message: "sample message".to_string(),
        author: "sample-author".to_string(),
        date: Utc::now(),
        is_locked: false,
    }
}

fn sample_flake() -> FlakeData {
    FlakeData {
        path: PathBuf::from("/tmp/flake"),
        inputs: vec![FlakeInput::Git(sample_git_input("nixpkgs"))],
    }
}

#[test]
fn smoke_flow_load_update_changelog_and_lock() {
    let mut state = AppState::Loading;

    let loaded = TaskResult::FlakeLoaded {
        effect_id: 1,
        result: Ok(sample_flake()),
    };
    let transition = reduce(&mut state, AppEvent::TaskResult(&loaded));
    assert!(matches!(state, AppState::List(_)));
    assert!(matches!(
        transition.effects.as_slice(),
        [Effect::CheckUpdates { .. }]
    ));

    let update_action = Action::UpdateSelected(vec!["nixpkgs".to_string()]);
    let transition = reduce(&mut state, AppEvent::Action(&update_action));
    assert!(matches!(
        transition.effects.as_slice(),
        [Effect::Update { .. }]
    ));
    assert_eq!(
        transition.status,
        StatusCommand::Info("Updating 1 input(s)...".to_string())
    );
    if let AppState::List(list) = &state {
        assert!(list.busy);
    }

    let updated = TaskResult::UpdateComplete {
        effect_id: 2,
        result: Ok(()),
    };
    let transition = reduce(&mut state, AppEvent::TaskResult(&updated));
    assert!(matches!(transition.effects.as_slice(), [Effect::LoadFlake]));

    let open = Action::OpenChangelog { input_idx: 0 };
    let transition = reduce(&mut state, AppEvent::Action(&open));
    assert!(matches!(state, AppState::LoadingChangelog(_)));
    assert!(matches!(
        transition.effects.as_slice(),
        [Effect::LoadChangelog { .. }]
    ));

    let parent = match state {
        AppState::LoadingChangelog(ref list) => list.clone(),
        _ => ListState::new(sample_flake()),
    };
    let changelog_result = TaskResult::ChangelogLoaded {
        effect_id: 3,
        result: Box::new(Ok(melt::app::ChangelogLoadedData {
            input: sample_git_input("nixpkgs"),
            data: ChangelogData {
                commits: vec![sample_commit("deadbeef")],
                locked_idx: None,
            },
            parent_list: parent,
        })),
    };
    let transition = reduce(&mut state, AppEvent::TaskResult(&changelog_result));
    assert!(matches!(state, AppState::Changelog(_)));
    assert_eq!(transition.status, StatusCommand::Clear);

    let show_confirm = Action::ChangelogShowConfirm;
    let _ = reduce(&mut state, AppEvent::Action(&show_confirm));

    let lock = Action::ConfirmLock {
        input_name: "nixpkgs".to_string(),
        lock_url: "github:NixOS/nixpkgs/deadbeef".to_string(),
    };
    let transition = reduce(&mut state, AppEvent::Action(&lock));
    assert!(matches!(
        transition.effects.as_slice(),
        [Effect::Lock { .. }]
    ));
}
