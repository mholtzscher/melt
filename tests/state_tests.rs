use std::path::PathBuf;

use chrono::Utc;
use melt::app::{ChangelogState, ListState};
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

fn sample_commit(sha: &str, is_locked: bool) -> Commit {
    Commit {
        sha: sha.to_string(),
        message: "sample message".to_string(),
        author: "sample-author".to_string(),
        date: Utc::now(),
        is_locked,
    }
}

fn list_with_git_input(name: &str) -> ListState {
    let flake = FlakeData {
        path: PathBuf::from("/tmp/flake"),
        inputs: vec![FlakeInput::Git(sample_git_input(name))],
    };
    ListState::new(flake)
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
