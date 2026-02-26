pub mod fallback;
pub mod github;
pub mod gitlab;

use std::path::PathBuf;
use std::sync::Arc;

use reqwest::Client;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::app::ports::{GitPort, PortFuture, StatusCallback};
use crate::config::ServiceConfig;
use crate::error::GitError;
use crate::model::{ChangelogData, ForgeType, GitInput, UpdateStatus};

use self::fallback::Git2Client;
use self::github::GithubClient;
use self::gitlab::GitlabClient;

/// Service for git operations - uses APIs where possible, falls back to git2
#[derive(Clone)]
pub struct GitService {
    cache_dir: PathBuf,
    cancel_token: CancellationToken,
    /// Semaphore to limit concurrent operations
    semaphore: Arc<Semaphore>,
    /// HTTP client for API requests
    client: Client,
    /// GitHub token for API authentication (optional)
    github_token: Option<String>,
    timeouts: crate::config::Timeouts,
}

impl GitService {
    /// Create a new GitService
    pub fn new(cancel_token: CancellationToken) -> Self {
        Self::new_with_config(cancel_token, ServiceConfig::default())
    }

    pub fn new_with_config(cancel_token: CancellationToken, config: ServiceConfig) -> Self {
        let cache_dir = get_cache_dir();
        let timeouts = config.timeouts.clone();
        let client = Client::builder()
            .timeout(timeouts.http_request)
            .user_agent("melt/0.1.0")
            .build()
            .unwrap_or_default();

        let github_token = std::env::var("GITHUB_TOKEN")
            .or_else(|_| std::env::var("GH_TOKEN"))
            .ok();

        Self {
            cache_dir,
            cancel_token,
            semaphore: Arc::new(Semaphore::new(config.git_concurrency)),
            client,
            github_token,
            timeouts,
        }
    }

    /// Check for updates on multiple inputs
    pub async fn check_updates<F>(
        &self,
        inputs: &[GitInput],
        mut on_status: F,
    ) -> Result<(), GitError>
    where
        F: FnMut(&str, UpdateStatus) + Send,
    {
        debug!(
            total = inputs.len(),
            "Checking for updates"
        );

        for input in inputs {
            on_status(&input.name, UpdateStatus::Checking);
        }

        let mut join_set = JoinSet::new();

        for input in inputs.iter().cloned() {
            if self.cancel_token.is_cancelled() {
                break;
            }

            let service = self.clone();
            let semaphore = self.semaphore.clone();

            join_set.spawn(async move {
                let name = input.name.clone();
                let _permit = match semaphore.acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return (
                            name,
                            UpdateStatus::Error("Failed to acquire semaphore".to_string()),
                        );
                    }
                };

                let status = match service.check_input_updates(&input).await {
                    Ok(0) => UpdateStatus::UpToDate,
                    Ok(count) => {
                        debug!(input = %name, behind = count, "Updates available");
                        UpdateStatus::Behind(count)
                    }
                    Err(e) => {
                        warn!(input = %name, error = %e, "Failed to check input");
                        UpdateStatus::Error(e.to_string())
                    }
                };

                (name, status)
            });
        }

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    join_set.abort_all();
                    break;
                }
                next = join_set.join_next() => {
                    match next {
                        Some(Ok((name, status))) => on_status(&name, status),
                        Some(Err(e)) if e.is_cancelled() => {}
                        Some(Err(e)) => warn!(error = %e, "Update check task failed"),
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }

    async fn check_input_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let fallback = Git2Client {
            cache_dir: &self.cache_dir,
            cancel_token: &self.cancel_token,
            timeout: self.timeouts.git_update_check,
        };

        match input.forge_type {
            ForgeType::GitHub => {
                let client = GithubClient {
                    client: &self.client,
                    token: self.github_token.as_deref(),
                };
                match client.check_updates(input).await {
                    Ok(count) => Ok(count),
                    Err(GitError::ApiFallbackRequired) => fallback.check_updates(input).await,
                    Err(e) => Err(e),
                }
            }
            ForgeType::GitLab => {
                let client = GitlabClient {
                    client: &self.client,
                };
                match client.check_updates(input).await {
                    Ok(count) => Ok(count),
                    Err(GitError::ApiFallbackRequired) => fallback.check_updates(input).await,
                    Err(e) => Err(e),
                }
            }
            _ => fallback.check_updates(input).await,
        }
    }

    pub async fn get_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        debug!(input = %input.name, forge = ?input.forge_type, "Loading changelog");

        let fallback = Git2Client {
            cache_dir: &self.cache_dir,
            cancel_token: &self.cancel_token,
            timeout: self.timeouts.git_changelog,
        };

        match input.forge_type {
            ForgeType::GitHub => {
                let client = GithubClient {
                    client: &self.client,
                    token: self.github_token.as_deref(),
                };
                match client.get_changelog(input).await {
                    Ok(data) => Ok(data),
                    Err(GitError::ApiFallbackRequired) => fallback.get_changelog(input).await,
                    Err(e) => Err(e),
                }
            }
            ForgeType::GitLab => {
                let client = GitlabClient {
                    client: &self.client,
                };
                match client.get_changelog(input).await {
                    Ok(data) => Ok(data),
                    Err(GitError::ApiFallbackRequired) => fallback.get_changelog(input).await,
                    Err(e) => Err(e),
                }
            }
            _ => fallback.get_changelog(input).await,
        }
    }
}

/// Get the XDG cache directory for melt
fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("melt")
        .join("git")
}

impl GitPort for GitService {
    fn get_changelog<'a>(
        &'a self,
        input: &'a GitInput,
    ) -> PortFuture<'a, Result<ChangelogData, GitError>> {
        Box::pin(async move { self.get_changelog(input).await })
    }

    fn check_updates<'a>(
        &'a self,
        inputs: &'a [GitInput],
        mut on_status: StatusCallback<'a>,
    ) -> PortFuture<'a, Result<(), GitError>> {
        Box::pin(async move {
            self.check_updates(inputs, move |name, status| on_status(name, status))
                .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use self::fallback::get_clone_url;

    #[test]
    fn test_get_clone_url_wrapper() {
        let input = GitInput {
            name: "nixpkgs".to_string(),
            owner: "NixOS".to_string(),
            repo: "nixpkgs".to_string(),
            forge_type: ForgeType::GitHub,
            host: None,
            reference: Some("nixos-unstable".to_string()),
            rev: "abc1234".to_string(),
            last_modified: 0,
            url: "github:NixOS/nixpkgs".to_string(),
        };

        assert_eq!(
            get_clone_url(&input),
            "https://github.com/NixOS/nixpkgs.git"
        );
    }

    #[test]
    fn test_get_clone_url_generic_uses_input_url() {
        let input = GitInput {
            name: "emacs".to_string(),
            owner: "gnu".to_string(),
            repo: "emacs".to_string(),
            forge_type: ForgeType::Generic,
            host: None,
            reference: None,
            rev: "abc1234".to_string(),
            last_modified: 0,
            url: "https://git.savannah.gnu.org/git/emacs.git".to_string(),
        };

        assert_eq!(
            get_clone_url(&input),
            "https://git.savannah.gnu.org/git/emacs.git"
        );
    }

    #[test]
    fn test_get_clone_url_generic_strips_git_prefix() {
        let input = GitInput {
            name: "forgejo".to_string(),
            owner: "forgejo".to_string(),
            repo: "forgejo".to_string(),
            forge_type: ForgeType::Generic,
            host: None,
            reference: None,
            rev: "abc1234".to_string(),
            last_modified: 0,
            url: "git+https://codeberg.org/forgejo/forgejo".to_string(),
        };

        assert_eq!(
            get_clone_url(&input),
            "https://codeberg.org/forgejo/forgejo"
        );
    }
}
