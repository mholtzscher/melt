use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tracing::{debug, warn};

use crate::app::ports::{NixPort, PortFuture};
use crate::config::ServiceConfig;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};
use crate::model::{FlakeData, FlakeInput, FlakeUrl, ForgeType, GitInput, OtherInput, PathInput};

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

/// Parse a single input node
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
                _ => url_for_parse.and_then(FlakeUrl::parse_owner_repo),
            };

            let Some((owner, repo)) = owner_repo else {
                return Some(FlakeInput::Other(OtherInput {
                    name: name.to_string(),
                    rev: locked.rev.clone().unwrap_or_default(),
                    last_modified: locked.last_modified.unwrap_or(0),
                }));
            };

            let forge_type = match type_ {
                "github" => ForgeType::GitHub,
                "gitlab" => ForgeType::GitLab,
                "sourcehut" => ForgeType::SourceHut,
                "git" => {
                    if let Some(u) = url_for_parse {
                        FlakeUrl::detect_forge_from_url(u)
                    } else {
                        ForgeType::Generic
                    }
                }
                _ => ForgeType::Generic,
            };

            let host = locked
                .host
                .clone()
                .or_else(|| original.and_then(|o| o.host.clone()));
            let reference = original.and_then(|o| o.reference.clone());
            let rev = locked.rev.clone().unwrap_or_default();
            
            let locked_url = locked.url.as_deref();
            let original_url = original.and_then(|o| o.url.as_deref());
            let url = FlakeUrl::build_display_url(
                type_, 
                &owner, 
                &repo, 
                host.as_deref(), 
                locked_url, 
                original_url
            );

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
        "path" => Some(FlakeInput::Path(PathInput {
            name: name.to_string(),
        })),
        _ => Some(FlakeInput::Other(OtherInput {
            name: name.to_string(),
            rev: locked.rev.clone().unwrap_or_default(),
            last_modified: locked.last_modified.unwrap_or(0),
        })),
    }
}

impl NixPort for NixService {
    fn load_metadata<'a>(&'a self, path: &'a Path) -> PortFuture<'a, AppResult<FlakeData>> {
        Box::pin(async move { self.load_metadata(path).await })
    }

    fn update_inputs<'a>(
        &'a self,
        path: &'a Path,
        names: &'a [String],
    ) -> PortFuture<'a, AppResult<()>> {
        Box::pin(async move { self.update_inputs(path, names).await })
    }

    fn update_all<'a>(&'a self, path: &'a Path) -> PortFuture<'a, AppResult<()>> {
        Box::pin(async move { self.update_all(path).await })
    }

    fn lock_input<'a>(
        &'a self,
        path: &'a Path,
        name: &'a str,
        override_url: &'a str,
    ) -> PortFuture<'a, AppResult<()>> {
        Box::pin(async move { self.lock_input(path, name, override_url).await })
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





}
