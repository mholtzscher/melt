//! Integration tests for melt-rs
//!
//! These tests use test fixtures from the original melt project.
//! They test metadata parsing without requiring nix to be installed.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

// Re-implement the parsing logic for testing without exposing internals
// This mirrors the NixService parsing

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeType {
    GitHub,
    GitLab,
    SourceHut,
    Codeberg,
    Gitea,
    Generic,
}

#[derive(Debug, Clone)]
pub struct GitInput {
    pub name: String,
    pub owner: String,
    pub repo: String,
    pub forge_type: ForgeType,
    pub host: Option<String>,
    pub reference: Option<String>,
    pub rev: String,
    pub last_modified: i64,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct PathInput {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct OtherInput {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub enum FlakeInput {
    Git(GitInput),
    Path(PathInput),
    Other(OtherInput),
}

impl FlakeInput {
    pub fn name(&self) -> &str {
        match self {
            FlakeInput::Git(g) => &g.name,
            FlakeInput::Path(p) => &p.name,
            FlakeInput::Other(o) => &o.name,
        }
    }
}

// JSON structures for parsing
#[derive(Debug, Deserialize)]
struct NixFlakeMetadata {
    description: Option<String>,
    #[serde(default)]
    locks: NixLocks,
}

#[derive(Debug, Deserialize, Default)]
struct NixLocks {
    #[serde(default)]
    nodes: HashMap<String, NixNode>,
    #[serde(default)]
    root: String,
}

#[derive(Debug, Deserialize, Default)]
struct NixNode {
    #[serde(default)]
    inputs: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    locked: Option<NixLocked>,
    #[serde(default)]
    original: Option<NixOriginal>,
}

#[derive(Debug, Deserialize, Default)]
struct NixLocked {
    #[serde(rename = "type", default)]
    type_: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    rev: Option<String>,
    #[serde(rename = "lastModified", default)]
    last_modified: Option<i64>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    host: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct NixOriginal {
    #[serde(rename = "type", default)]
    type_: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(rename = "ref", default)]
    reference: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    host: Option<String>,
}

fn detect_forge_type(type_: &str, locked: &NixLocked, original: Option<&NixOriginal>) -> ForgeType {
    match type_ {
        "github" => ForgeType::GitHub,
        "gitlab" => ForgeType::GitLab,
        "sourcehut" => ForgeType::SourceHut,
        "git" => {
            let url = locked
                .url
                .as_deref()
                .or_else(|| original.and_then(|o| o.url.as_deref()))
                .unwrap_or("");

            if url.contains("github.com") {
                ForgeType::GitHub
            } else if url.contains("gitlab") {
                ForgeType::GitLab
            } else if url.contains("sr.ht") || url.contains("sourcehut") {
                ForgeType::SourceHut
            } else if url.contains("codeberg.org") {
                ForgeType::Codeberg
            } else if url.contains("gitea") || url.contains("forgejo") {
                ForgeType::Gitea
            } else {
                ForgeType::Generic
            }
        }
        _ => ForgeType::Generic,
    }
}

fn build_url(
    type_: &str,
    owner: &str,
    repo: &str,
    host: Option<&str>,
    locked: &NixLocked,
    original: Option<&NixOriginal>,
) -> String {
    match type_ {
        "github" => format!("github:{}/{}", owner, repo),
        "gitlab" => {
            if let Some(h) = host {
                if h != "gitlab.com" {
                    return format!("gitlab:{}/{} ({})", owner, repo, h);
                }
            }
            format!("gitlab:{}/{}", owner, repo)
        }
        "sourcehut" => {
            let o = if owner.starts_with('~') {
                owner.to_string()
            } else {
                format!("~{}", owner)
            };
            format!("sourcehut:{}/{}", o, repo)
        }
        "git" => locked
            .url
            .clone()
            .or_else(|| original.and_then(|o| o.url.clone()))
            .unwrap_or_else(|| format!("git:{}/{}", owner, repo)),
        _ => "unknown".to_string(),
    }
}

fn parse_input(name: &str, node: &NixNode) -> Option<FlakeInput> {
    let locked = node.locked.as_ref()?;
    let original = node.original.as_ref();

    let type_ = locked
        .type_
        .as_deref()
        .or_else(|| original.and_then(|o| o.type_.as_deref()))
        .unwrap_or("other");

    match type_ {
        "github" | "gitlab" | "sourcehut" | "git" => {
            let forge_type = detect_forge_type(type_, locked, original);
            let owner = locked
                .owner
                .clone()
                .or_else(|| original.and_then(|o| o.owner.clone()))
                .unwrap_or_default();
            let repo = locked
                .repo
                .clone()
                .or_else(|| original.and_then(|o| o.repo.clone()))
                .unwrap_or_default();
            let host = locked
                .host
                .clone()
                .or_else(|| original.and_then(|o| o.host.clone()));
            let reference = original.and_then(|o| o.reference.clone());
            let rev = locked.rev.clone().unwrap_or_default();
            let url = build_url(type_, &owner, &repo, host.as_deref(), locked, original);

            Some(FlakeInput::Git(GitInput {
                name: name.to_string(),
                owner,
                repo,
                forge_type,
                host,
                reference,
                rev,
                last_modified: locked.last_modified.unwrap_or(0),
                url,
            }))
        }
        "path" => {
            let path = locked
                .path
                .clone()
                .or_else(|| original.and_then(|o| o.path.clone()))
                .unwrap_or_default();

            Some(FlakeInput::Path(PathInput {
                name: name.to_string(),
                path,
            }))
        }
        _ => {
            let url = locked
                .url
                .clone()
                .or_else(|| original.and_then(|o| o.url.clone()))
                .unwrap_or_else(|| "unknown".to_string());

            Some(FlakeInput::Other(OtherInput {
                name: name.to_string(),
                url,
            }))
        }
    }
}

fn parse_lock_file(content: &str) -> Vec<FlakeInput> {
    // Parse as a lock file directly (not metadata wrapper)
    let locks: NixLocks = serde_json::from_str(content).unwrap_or_default();

    let root_node = locks.nodes.get(&locks.root);
    root_node
        .and_then(|n| n.inputs.as_ref())
        .map(|inputs| {
            inputs
                .iter()
                .filter_map(|(name, value)| {
                    let node_name = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Array(arr) => arr.first()?.as_str()?.to_string(),
                        _ => return None,
                    };

                    let node = locks.nodes.get(&node_name)?;
                    parse_input(name, node)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_test_data_path() -> PathBuf {
    // Navigate from melt-rs to melt/test-data
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("melt")
        .join("test-data")
}

#[test]
fn test_parse_minimal_flake() {
    let lock_path = get_test_data_path().join("minimal").join("flake.lock");
    let content = std::fs::read_to_string(&lock_path).expect("Failed to read minimal flake.lock");

    let inputs = parse_lock_file(&content);

    assert_eq!(inputs.len(), 1, "Minimal flake should have 1 input");

    let nixpkgs = inputs
        .iter()
        .find(|i| i.name() == "nixpkgs")
        .expect("Should have nixpkgs input");

    match nixpkgs {
        FlakeInput::Git(g) => {
            assert_eq!(g.owner.to_lowercase(), "nixos");
            assert_eq!(g.repo, "nixpkgs");
            assert_eq!(g.forge_type, ForgeType::GitHub);
            assert!(g.rev.len() >= 7, "Rev should be a valid commit hash");
        }
        _ => panic!("nixpkgs should be a Git input"),
    }
}

#[test]
fn test_parse_all_forges_flake() {
    let lock_path = get_test_data_path().join("all-forges").join("flake.lock");
    let content =
        std::fs::read_to_string(&lock_path).expect("Failed to read all-forges flake.lock");

    let inputs = parse_lock_file(&content);

    // Should have multiple inputs
    assert!(
        inputs.len() >= 4,
        "all-forges should have at least 4 inputs, got {}",
        inputs.len()
    );

    // Check GitHub input
    let nixpkgs = inputs.iter().find(|i| i.name() == "nixpkgs");
    assert!(nixpkgs.is_some(), "Should have nixpkgs input");
    if let Some(FlakeInput::Git(g)) = nixpkgs {
        assert_eq!(g.forge_type, ForgeType::GitHub);
    }

    // Check GitLab input
    let gitlab = inputs.iter().find(|i| i.name() == "gitlab-example");
    assert!(gitlab.is_some(), "Should have gitlab-example input");
    if let Some(FlakeInput::Git(g)) = gitlab {
        assert_eq!(g.forge_type, ForgeType::GitLab);
    }

    // Check SourceHut input
    let sourcehut = inputs.iter().find(|i| i.name() == "sourcehut-example");
    assert!(sourcehut.is_some(), "Should have sourcehut-example input");
    if let Some(FlakeInput::Git(g)) = sourcehut {
        assert_eq!(g.forge_type, ForgeType::SourceHut);
    }

    // Check path input
    let local = inputs.iter().find(|i| i.name() == "local-example");
    assert!(local.is_some(), "Should have local-example input");
    match local {
        Some(FlakeInput::Path(p)) => {
            assert!(p.path.contains("local"), "Path should contain 'local'");
        }
        _ => panic!("local-example should be a Path input"),
    }
}

#[test]
fn test_forge_type_detection_from_url() {
    let locked = NixLocked {
        type_: Some("git".to_string()),
        url: Some("https://codeberg.org/forgejo/forgejo".to_string()),
        owner: None,
        repo: None,
        rev: None,
        last_modified: None,
        path: None,
        host: None,
    };

    assert_eq!(detect_forge_type("git", &locked, None), ForgeType::Codeberg);

    let locked_github = NixLocked {
        type_: Some("git".to_string()),
        url: Some("https://github.com/owner/repo".to_string()),
        ..Default::default()
    };

    assert_eq!(
        detect_forge_type("git", &locked_github, None),
        ForgeType::GitHub
    );

    let locked_generic = NixLocked {
        type_: Some("git".to_string()),
        url: Some("https://custom-host.example.com/repo".to_string()),
        ..Default::default()
    };

    assert_eq!(
        detect_forge_type("git", &locked_generic, None),
        ForgeType::Generic
    );
}

#[test]
fn test_github_heavy_flake() {
    let lock_path = get_test_data_path().join("github-heavy").join("flake.lock");
    let content =
        std::fs::read_to_string(&lock_path).expect("Failed to read github-heavy flake.lock");

    let inputs = parse_lock_file(&content);

    // All inputs should be GitHub
    for input in &inputs {
        if let FlakeInput::Git(g) = input {
            assert_eq!(
                g.forge_type,
                ForgeType::GitHub,
                "Input {} should be GitHub, not {:?}",
                g.name,
                g.forge_type
            );
        }
    }
}

#[test]
fn test_nixos_config_flake() {
    let lock_path = get_test_data_path().join("nixos-config").join("flake.lock");

    // This test is optional - skip if fixture doesn't exist
    if !lock_path.exists() {
        return;
    }

    let content =
        std::fs::read_to_string(&lock_path).expect("Failed to read nixos-config flake.lock");

    let inputs = parse_lock_file(&content);

    // Should parse without errors
    assert!(!inputs.is_empty(), "nixos-config should have inputs");
}

#[test]
fn test_empty_inputs_handling() {
    // Test parsing a minimal lock file with no inputs
    let empty_lock = r#"{
        "nodes": {
            "root": {
                "inputs": {}
            }
        },
        "root": "root",
        "version": 7
    }"#;

    let inputs = parse_lock_file(empty_lock);
    assert!(inputs.is_empty(), "Empty lock should have no inputs");
}

#[test]
fn test_malformed_json_handling() {
    let bad_json = "{ invalid json }";
    let inputs = parse_lock_file(bad_json);
    // Should not panic, just return empty
    assert!(inputs.is_empty());
}

#[test]
fn test_missing_locked_fields() {
    // Test handling of incomplete locked data
    let lock_json = r#"{
        "nodes": {
            "nixpkgs": {
                "locked": {
                    "type": "github",
                    "owner": "NixOS",
                    "repo": "nixpkgs"
                }
            },
            "root": {
                "inputs": {
                    "nixpkgs": "nixpkgs"
                }
            }
        },
        "root": "root",
        "version": 7
    }"#;

    let inputs = parse_lock_file(lock_json);
    assert_eq!(inputs.len(), 1);

    if let Some(FlakeInput::Git(g)) = inputs.first() {
        // Missing fields should be empty strings
        assert!(g.rev.is_empty());
        assert_eq!(g.last_modified, 0);
        assert!(g.reference.is_none());
    } else {
        panic!("Expected Git input");
    }
}

#[test]
fn test_indirect_input_reference() {
    // Test input that references another input via array notation
    let lock_json = r#"{
        "nodes": {
            "flake-utils": {
                "locked": {
                    "type": "github",
                    "owner": "numtide",
                    "repo": "flake-utils",
                    "rev": "abcdef1234567890",
                    "lastModified": 1700000000
                }
            },
            "nixpkgs": {
                "locked": {
                    "type": "github",
                    "owner": "NixOS",
                    "repo": "nixpkgs",
                    "rev": "1234567890abcdef",
                    "lastModified": 1700000000
                }
            },
            "some-flake": {
                "inputs": {
                    "nixpkgs": ["nixpkgs"]
                },
                "locked": {
                    "type": "github",
                    "owner": "owner",
                    "repo": "some-flake",
                    "rev": "deadbeef12345678",
                    "lastModified": 1700000000
                }
            },
            "root": {
                "inputs": {
                    "flake-utils": "flake-utils",
                    "nixpkgs": "nixpkgs",
                    "some-flake": "some-flake"
                }
            }
        },
        "root": "root",
        "version": 7
    }"#;

    let inputs = parse_lock_file(lock_json);
    assert_eq!(inputs.len(), 3, "Should have 3 inputs");

    // Verify all three are parsed
    let names: Vec<&str> = inputs.iter().map(|i| i.name()).collect();
    assert!(names.contains(&"flake-utils"));
    assert!(names.contains(&"nixpkgs"));
    assert!(names.contains(&"some-flake"));
}

#[test]
fn test_url_building() {
    let locked = NixLocked::default();

    // GitHub URL
    let url = build_url("github", "NixOS", "nixpkgs", None, &locked, None);
    assert_eq!(url, "github:NixOS/nixpkgs");

    // GitLab URL
    let url = build_url("gitlab", "owner", "repo", None, &locked, None);
    assert_eq!(url, "gitlab:owner/repo");

    // GitLab with custom host
    let url = build_url(
        "gitlab",
        "owner",
        "repo",
        Some("gitlab.gnome.org"),
        &locked,
        None,
    );
    assert_eq!(url, "gitlab:owner/repo (gitlab.gnome.org)");

    // SourceHut URL (without tilde)
    let url = build_url("sourcehut", "user", "repo", None, &locked, None);
    assert_eq!(url, "sourcehut:~user/repo");

    // SourceHut URL (with tilde)
    let url = build_url("sourcehut", "~user", "repo", None, &locked, None);
    assert_eq!(url, "sourcehut:~user/repo");
}

#[test]
fn test_input_name_extraction() {
    let git_input = FlakeInput::Git(GitInput {
        name: "nixpkgs".to_string(),
        owner: "NixOS".to_string(),
        repo: "nixpkgs".to_string(),
        forge_type: ForgeType::GitHub,
        host: None,
        reference: Some("nixos-unstable".to_string()),
        rev: "abc1234".to_string(),
        last_modified: 1234567890,
        url: "github:NixOS/nixpkgs".to_string(),
    });

    assert_eq!(git_input.name(), "nixpkgs");

    let path_input = FlakeInput::Path(PathInput {
        name: "local".to_string(),
        path: "./local".to_string(),
    });

    assert_eq!(path_input.name(), "local");
}

#[test]
fn test_reference_parsing() {
    // Test that branch references are parsed correctly
    let lock_json = r#"{
        "nodes": {
            "nixpkgs": {
                "locked": {
                    "type": "github",
                    "owner": "NixOS",
                    "repo": "nixpkgs",
                    "rev": "abc1234def5678",
                    "lastModified": 1700000000
                },
                "original": {
                    "type": "github",
                    "owner": "NixOS",
                    "repo": "nixpkgs",
                    "ref": "nixos-unstable"
                }
            },
            "root": {
                "inputs": {
                    "nixpkgs": "nixpkgs"
                }
            }
        },
        "root": "root",
        "version": 7
    }"#;

    let inputs = parse_lock_file(lock_json);
    assert_eq!(inputs.len(), 1);

    if let Some(FlakeInput::Git(g)) = inputs.first() {
        assert_eq!(g.reference, Some("nixos-unstable".to_string()));
        assert_eq!(g.rev, "abc1234def5678");
    } else {
        panic!("Expected Git input");
    }
}
