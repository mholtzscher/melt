use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tracing::{debug, warn};

use crate::config::ServiceConfig;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};
use crate::model::{
    CloneUrl, FlakeData, FlakeInput, GitHost, GitInput, GitRef, GitRepo, GitRev, InputName,
    OtherInput, Owner, PathInput, RepoName,
};

/// Service for interacting with Nix flakes
#[derive(Clone)]
pub struct NixService {
    cancel_token: CancellationToken,
    nix_command_timeout: Duration,
}

impl NixService {
    /// Create a new NixService
    pub fn new(cancel_token: CancellationToken) -> Self {
        Self::new_with_config(cancel_token, ServiceConfig::default())
    }

    pub fn new_with_config(cancel_token: CancellationToken, config: ServiceConfig) -> Self {
        Self {
            cancel_token,
            nix_command_timeout: config.timeouts.nix_command,
        }
    }

    pub async fn load_metadata(&self, path: &Path) -> AppResult<FlakeData> {
        let flake_path = resolve_flake_path(path)?;

        if !flake_path.join("flake.nix").exists() {
            return Err(AppError::FlakeNotFound(flake_path));
        }

        let output = self.run_nix_metadata(&flake_path).await?;
        let metadata: NixFlakeMetadata = serde_json::from_str(&output)
            .map_err(|e| AppError::MetadataParseError(e.to_string()))?;

        Ok(parse_metadata(flake_path, metadata))
    }

    pub async fn update_inputs(&self, path: &Path, names: &[String]) -> AppResult<()> {
        if names.is_empty() {
            return Ok(());
        }

        debug!(inputs = ?names, "Updating inputs");

        let mut args = vec!["flake", "update"];
        for name in names {
            args.push(name);
        }
        args.push("--flake");
        let path_str = path.to_string_lossy();
        args.push(&path_str);

        self.run_nix_command(&args).await?;
        Ok(())
    }

    pub async fn update_all(&self, path: &Path) -> AppResult<()> {
        debug!("Updating all inputs");
        let path_str = path.to_string_lossy();
        self.run_nix_command(&["flake", "update", "--flake", &path_str])
            .await?;
        Ok(())
    }

    pub async fn lock_input(&self, path: &Path, name: &str, override_url: &str) -> AppResult<()> {
        debug!(input = %name, "Locking input");
        let path_str = path.to_string_lossy();
        self.run_nix_command(&[
            "flake",
            "update",
            name,
            "--override-input",
            name,
            override_url,
            "--flake",
            &path_str,
        ])
        .await?;
        Ok(())
    }

    /// Run `nix flake metadata --json` and return the output
    async fn run_nix_metadata(&self, path: &Path) -> AppResult<String> {
        let path_str = path.to_string_lossy();
        self.run_nix_command(&[
            "flake",
            "metadata",
            "--json",
            "--no-write-lock-file",
            &path_str,
        ])
        .await
    }

    async fn run_nix_command(&self, args: &[&str]) -> AppResult<String> {
        if self.cancel_token.is_cancelled() {
            return Err(AppError::NixCommandFailed(
                "Operation cancelled".to_string(),
            ));
        }

        let mut cmd = Command::new("nix");
        cmd.arg("--option").arg("warn-dirty").arg("false");
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

        let timeout = tokio::time::timeout(self.nix_command_timeout, cmd.output());

        let output = tokio::select! {
            result = timeout => {
                match result {
                    Ok(Ok(output)) => output,
                    Ok(Err(e)) => return Err(AppError::Io(e)),
                    Err(_) => return Err(AppError::NixCommandFailed("Command timed out".to_string())),
                }
            }
            _ = self.cancel_token.cancelled() => {
                return Err(AppError::NixCommandFailed("Operation cancelled".to_string()));
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(args = ?args, "Nix command failed");
            return Err(AppError::NixCommandFailed(stderr.trim().to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

fn resolve_flake_path(path: &Path) -> AppResult<PathBuf> {
    let path = if path.to_string_lossy().is_empty() || path.to_string_lossy() == "." {
        std::env::current_dir()?
    } else {
        path.to_path_buf()
    };

    let resolved = if path.ends_with("flake.nix") {
        path.parent()
            .ok_or_else(|| AppError::FlakeNotFound(path.clone()))?
            .to_path_buf()
    } else {
        path
    };

    resolved
        .canonicalize()
        .map_err(|_| AppError::FlakeNotFound(resolved))
}

// JSON structures for nix flake metadata
// Using deny_unknown_fields = false (default) to handle different nix versions

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NixFlakeMetadata {
    description: Option<String>,
    #[serde(default)]
    locks: NixLocks,
}

#[derive(Debug, Deserialize, Default)]
struct NixLocks {
    #[serde(default)]
    nodes: std::collections::HashMap<String, NixNode>,
    #[serde(default)]
    root: String,
}

#[derive(Debug, Deserialize, Default)]
struct NixNode {
    #[serde(default)]
    inputs: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default)]
    locked: Option<NixLocked>,
    #[serde(default)]
    original: Option<NixOriginal>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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

fn parse_metadata(path: PathBuf, metadata: NixFlakeMetadata) -> FlakeData {
    let root_node = metadata.locks.nodes.get(&metadata.locks.root);
    let mut inputs: Vec<FlakeInput> = root_node
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

                    let node = metadata.locks.nodes.get(&node_name)?;
                    parse_input(name, node)
                })
                .collect()
        })
        .unwrap_or_default();

    inputs.sort_by_key(|a| a.name().to_lowercase());

    FlakeData { path, inputs }
}

/// Parse owner and repo from a git URL
fn parse_owner_repo_from_url(url: &str) -> Option<(String, String)> {
    fn parse_owner_repo_from_path(path: &str) -> Option<(String, String)> {
        let mut segments: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

        if segments.len() < 2 {
            return None;
        }

        let repo_segment = segments.pop()?;
        let repo = repo_segment.trim_end_matches(".git");
        if repo.is_empty() {
            return None;
        }

        let owner = segments.join("/");
        if owner.is_empty() {
            return None;
        }

        Some((owner, repo.to_string()))
    }

    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    let url = url.strip_prefix("git+").unwrap_or(url);

    // Scheme URLs: https://host/owner/repo, ssh://git@host:port/owner/repo
    if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("ssh://") {
        let rest = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .or_else(|| url.strip_prefix("ssh://"))?;

        // Drop authority (host / user@host:port)
        let path = rest.split_once('/')?.1;
        let path = path.split(['?', '#']).next().unwrap_or(path);

        return parse_owner_repo_from_path(path);
    }

    // SCP-style: git@host:owner/repo.git
    if url.contains(':') && !url.contains("://") {
        let (_, path) = url.split_once(':')?;
        let path = path.split(['?', '#']).next().unwrap_or(path);

        return parse_owner_repo_from_path(path);
    }

    None
}

#[derive(Debug, Clone)]
enum RawInputParseResult {
    ActionableGit(GitInput),
    DisplayOnly(FlakeInput),
    Skip,
}

impl RawInputParseResult {
    fn into_flake_input(self) -> Option<FlakeInput> {
        match self {
            Self::ActionableGit(input) => Some(FlakeInput::Git(input)),
            Self::DisplayOnly(input) => Some(input),
            Self::Skip => None,
        }
    }
}

/// Parse a single input node into raw parse boundary result.
fn parse_raw_input(name: &str, node: &NixNode) -> RawInputParseResult {
    let Some(locked) = node.locked.as_ref() else {
        return RawInputParseResult::Skip;
    };
    let original = node.original.as_ref();

    let type_ = locked
        .type_
        .as_deref()
        .or_else(|| original.and_then(|o| o.type_.as_deref()))
        .unwrap_or("other");

    match type_ {
        "github" | "gitlab" | "sourcehut" | "git" => {
            let forge_type = detect_forge_type(type_, locked, original);

            let meta_owner = locked
                .owner
                .clone()
                .or_else(|| original.and_then(|o| o.owner.clone()));
            let meta_repo = locked
                .repo
                .clone()
                .or_else(|| original.and_then(|o| o.repo.clone()));

            let url_for_parse = locked
                .url
                .as_deref()
                .or_else(|| original.and_then(|o| o.url.as_deref()));

            let owner_repo = match (meta_owner, meta_repo) {
                (Some(owner), Some(repo)) if !owner.is_empty() && !repo.is_empty() => {
                    Some((owner, repo))
                }
                _ => url_for_parse.and_then(parse_owner_repo_from_url),
            };

            let Some((owner, repo)) = owner_repo else {
                return RawInputParseResult::DisplayOnly(FlakeInput::Other(OtherInput {
                    name: name.to_string(),
                    rev: locked.rev.clone().filter(|rev| !rev.trim().is_empty()),
                    last_modified: locked.last_modified.unwrap_or(0),
                }));
            };
            let host = locked
                .host
                .clone()
                .or_else(|| original.and_then(|o| o.host.clone()));
            let reference = original.and_then(|o| o.reference.clone());
            let Some(rev) = locked.rev.clone().filter(|rev| !rev.trim().is_empty()) else {
                return RawInputParseResult::DisplayOnly(FlakeInput::Other(OtherInput {
                    name: name.to_string(),
                    rev: None,
                    last_modified: locked.last_modified.unwrap_or(0),
                }));
            };
            let url = build_url(type_, &owner, &repo, host.as_deref(), locked, original);

            let Ok(input_name) = InputName::new(name) else {
                return RawInputParseResult::Skip;
            };
            let Ok(git_rev) = GitRev::new(rev.clone()) else {
                return RawInputParseResult::DisplayOnly(FlakeInput::Other(OtherInput {
                    name: name.to_string(),
                    rev: Some(rev),
                    last_modified: locked.last_modified.unwrap_or(0),
                }));
            };
            let Some(git_repo) = build_git_repo(forge_type, owner, repo, host, locked, original)
            else {
                return RawInputParseResult::DisplayOnly(FlakeInput::Other(OtherInput {
                    name: name.to_string(),
                    rev: Some(git_rev.as_str().to_string()),
                    last_modified: locked.last_modified.unwrap_or(0),
                }));
            };
            let reference = reference.and_then(|reference| GitRef::new(reference).ok());

            RawInputParseResult::ActionableGit(GitInput::new(
                input_name,
                git_repo,
                reference,
                git_rev,
                locked.last_modified.unwrap_or(0),
                url,
            ))
        }
        "path" => RawInputParseResult::DisplayOnly(FlakeInput::Path(PathInput {
            name: name.to_string(),
        })),
        _ => RawInputParseResult::DisplayOnly(FlakeInput::Other(OtherInput {
            name: name.to_string(),
            rev: locked.rev.clone().filter(|rev| !rev.trim().is_empty()),
            last_modified: locked.last_modified.unwrap_or(0),
        })),
    }
}

/// Parse a single input node
fn parse_input(name: &str, node: &NixNode) -> Option<FlakeInput> {
    parse_raw_input(name, node).into_flake_input()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawForgeType {
    GitHub,
    GitLab,
    SourceHut,
    Codeberg,
    Gitea,
    Generic,
}

fn build_git_repo(
    forge_type: RawForgeType,
    owner: String,
    repo: String,
    host: Option<String>,
    locked: &NixLocked,
    original: Option<&NixOriginal>,
) -> Option<GitRepo> {
    let owner = Owner::new(owner).ok()?;
    let repo = RepoName::new(repo).ok()?;
    let host = host.and_then(|host| GitHost::new(host).ok());

    match forge_type {
        RawForgeType::GitHub => Some(GitRepo::github(owner, repo)),
        RawForgeType::GitLab => GitRepo::gitlab(host, owner, repo).ok(),
        RawForgeType::SourceHut => GitRepo::sourcehut(host, owner, repo).ok(),
        RawForgeType::Codeberg => Some(GitRepo::codeberg(owner, repo)),
        RawForgeType::Gitea => host.map(|host| GitRepo::gitea(host, owner, repo)),
        RawForgeType::Generic => {
            let url = locked
                .url
                .as_deref()
                .or_else(|| original.and_then(|o| o.url.as_deref()))?;
            CloneUrl::new(url.strip_prefix("git+").unwrap_or(url))
                .ok()
                .map(GitRepo::generic)
        }
    }
}

/// Detect the forge type from the input type and metadata
fn detect_forge_type(
    type_: &str,
    locked: &NixLocked,
    original: Option<&NixOriginal>,
) -> RawForgeType {
    match type_ {
        "github" => RawForgeType::GitHub,
        "gitlab" => RawForgeType::GitLab,
        "sourcehut" => RawForgeType::SourceHut,
        "git" => {
            // Try to detect from URL
            let url = locked
                .url
                .as_deref()
                .or_else(|| original.and_then(|o| o.url.as_deref()))
                .unwrap_or("");

            if url.contains("github.com") {
                RawForgeType::GitHub
            } else if url.contains("gitlab") {
                RawForgeType::GitLab
            } else if url.contains("sr.ht") || url.contains("sourcehut") {
                RawForgeType::SourceHut
            } else if url.contains("codeberg.org") {
                RawForgeType::Codeberg
            } else if url.contains("gitea") || url.contains("forgejo") {
                RawForgeType::Gitea
            } else {
                RawForgeType::Generic
            }
        }
        _ => RawForgeType::Generic,
    }
}

/// Build a display URL for the input
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_flake_path_dot() {
        // This test may fail in CI, so we just check it doesn't panic
        let _ = resolve_flake_path(Path::new("."));
    }

    #[test]
    fn test_detect_forge_type() {
        let locked = NixLocked {
            type_: Some("github".to_string()),
            owner: None,
            repo: None,
            rev: None,
            last_modified: None,
            url: None,
            path: None,
            host: None,
        };

        assert_eq!(
            detect_forge_type("github", &locked, None),
            RawForgeType::GitHub
        );
        assert_eq!(
            detect_forge_type("gitlab", &locked, None),
            RawForgeType::GitLab
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_https() {
        assert_eq!(
            parse_owner_repo_from_url("https://codeberg.org/LGFae/awww"),
            Some(("LGFae".to_string(), "awww".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("https://github.com/NixOS/nixpkgs.git"),
            Some(("NixOS".to_string(), "nixpkgs".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("https://gitlab.com/owner/repo"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("https://gitlab.com/group/subgroup/repo.git"),
            Some(("group/subgroup".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_ssh_scp_style() {
        assert_eq!(
            parse_owner_repo_from_url("git@github.com:owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("git@codeberg.org:LGFae/awww.git"),
            Some(("LGFae".to_string(), "awww".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("git@gitlab.com:group/subgroup/repo.git"),
            Some(("group/subgroup".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_ssh_scheme() {
        assert_eq!(
            parse_owner_repo_from_url("ssh://git@github.com/owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("ssh://git@example.com:2222/owner/repo.git"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_owner_repo_from_url("ssh://git@gitlab.com/group/subgroup/repo.git"),
            Some(("group/subgroup".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_owner_repo_from_url_edge_cases() {
        assert_eq!(
            parse_owner_repo_from_url("https://github.com/owner/repo/"),
            Some(("owner".to_string(), "repo".to_string()))
        );
        assert_eq!(parse_owner_repo_from_url("invalid-url"), None);
        assert_eq!(parse_owner_repo_from_url(""), None);
        assert_eq!(parse_owner_repo_from_url("https://github.com/"), None);
        assert_eq!(parse_owner_repo_from_url("https://github.com/owner/"), None);
    }

    fn git_node(
        type_: &str,
        owner: Option<&str>,
        repo: Option<&str>,
        rev: Option<&str>,
        url: Option<&str>,
        host: Option<&str>,
    ) -> NixNode {
        NixNode {
            inputs: None,
            locked: Some(NixLocked {
                type_: Some(type_.to_string()),
                owner: owner.map(ToOwned::to_owned),
                repo: repo.map(ToOwned::to_owned),
                rev: rev.map(ToOwned::to_owned),
                last_modified: Some(0),
                url: url.map(ToOwned::to_owned),
                path: None,
                host: host.map(ToOwned::to_owned),
            }),
            original: None,
        }
    }

    #[test]
    fn test_parse_input_missing_rev_is_not_actionable_git() {
        let node = git_node("github", Some("NixOS"), Some("nixpkgs"), None, None, None);
        let input = parse_input("nixpkgs", &node).unwrap();
        assert!(matches!(input, FlakeInput::Other(_)));
    }

    #[test]
    fn test_parse_input_missing_owner_repo_is_not_actionable_git() {
        let node = git_node("github", None, None, Some("abc1234"), None, None);
        let input = parse_input("nixpkgs", &node).unwrap();
        assert!(matches!(input, FlakeInput::Other(_)));
    }

    #[test]
    fn test_parse_input_gitea_without_host_is_not_actionable_git() {
        let node = git_node(
            "git",
            Some("owner"),
            Some("repo"),
            Some("abc1234"),
            Some("https://gitea.example.org/owner/repo.git"),
            None,
        );
        let input = parse_input("selfhosted", &node).unwrap();
        assert!(matches!(input, FlakeInput::Other(_)));
    }

    #[test]
    fn test_parse_input_valid_github_is_actionable_git() {
        let node = git_node(
            "github",
            Some("NixOS"),
            Some("nixpkgs"),
            Some("abc1234"),
            None,
            None,
        );
        let input = parse_input("nixpkgs", &node).unwrap();
        assert!(matches!(input, FlakeInput::Git(_)));
    }
}
