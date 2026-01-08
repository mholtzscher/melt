use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use reqwest::Client;
use tracing::{debug, warn};

use crate::config::ServiceConfig;
use serde::Deserialize;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::model::{ChangelogData, Commit, FlakeInput, ForgeType, GitInput, UpdateStatus};

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
        inputs: &[FlakeInput],
        mut on_status: F,
    ) -> Result<(), GitError>
    where
        F: FnMut(&str, UpdateStatus) + Send,
    {
        let git_inputs: Vec<&GitInput> = inputs
            .iter()
            .filter_map(|i| match i {
                FlakeInput::Git(g) => Some(g),
                _ => None,
            })
            .collect();

        debug!(
            total = inputs.len(),
            git_inputs = git_inputs.len(),
            "Checking for updates"
        );

        for input in &git_inputs {
            on_status(&input.name, UpdateStatus::Checking);
        }

        for input in git_inputs {
            if self.cancel_token.is_cancelled() {
                break;
            }

            let _permit =
                self.semaphore.acquire().await.map_err(|_| {
                    GitError::CloneFailed("Failed to acquire semaphore".to_string())
                })?;

            let status = match self.check_input_updates(input).await {
                Ok(0) => UpdateStatus::UpToDate,
                Ok(count) => {
                    debug!(input = %input.name, behind = count, "Updates available");
                    UpdateStatus::Behind(count)
                }
                Err(e) => {
                    warn!(input = %input.name, error = %e, "Failed to check input");
                    UpdateStatus::Error(e.to_string())
                }
            };

            on_status(&input.name, status);
        }

        Ok(())
    }

    async fn check_input_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        match input.forge_type {
            ForgeType::GitHub => self.check_github_updates(input).await,
            ForgeType::GitLab => self.check_gitlab_updates(input).await,
            ForgeType::SourceHut => self.check_sourcehut_updates(input).await,
            // For Codeberg/Gitea/Generic, fall back to git2 with timeout
            _ => self.check_git_updates(input).await,
        }
    }

    async fn check_github_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let branch = input.reference.as_deref().unwrap_or("HEAD");
        let url = format!(
            "https://api.github.com/repos/{}/{}/compare/{}...{}",
            input.owner, input.repo, input.rev, branch
        );

        let mut req = self.client.get(&url);
        if let Some(token) = &self.github_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let status = resp.status();

        if status.as_u16() == 403 || status.as_u16() == 429 {
            let remaining = resp
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);

            if remaining == 0 {
                warn!(input = %input.name, "GitHub API rate limit exceeded");
                return Err(GitError::NetworkError(
                    "GitHub API rate limit exceeded. Set GITHUB_TOKEN for higher limits."
                        .to_string(),
                ));
            }
        }

        if !status.is_success() {
            return self.check_git_updates(input).await;
        }

        #[derive(Deserialize)]
        struct CompareResponse {
            ahead_by: usize,
        }

        let data: CompareResponse = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        Ok(data.ahead_by)
    }

    async fn check_gitlab_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let host = input.host.as_deref().unwrap_or("gitlab.com");
        let branch = input.reference.as_deref().unwrap_or("HEAD");
        let project = format!("{}/{}", input.owner, input.repo);
        let encoded_project = urlencoding(&project);

        let url = format!(
            "https://{}/api/v4/projects/{}/repository/compare?from={}&to={}",
            host, encoded_project, input.rev, branch
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return self.check_git_updates(input).await;
        }

        #[derive(Deserialize)]
        struct CompareResponse {
            commits: Vec<serde_json::Value>,
        }

        let data: CompareResponse = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        Ok(data.commits.len())
    }

    async fn check_sourcehut_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let host = input.host.as_deref().unwrap_or("git.sr.ht");
        let owner = if input.owner.starts_with('~') {
            input.owner.clone()
        } else {
            format!("~{}", input.owner)
        };
        let branch = input.reference.as_deref().unwrap_or("HEAD");

        let url = format!(
            "https://{}/api/{}/{}/log/{}",
            host, owner, input.repo, branch
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return self.check_git_updates(input).await;
        }

        #[derive(Deserialize)]
        struct SrhtCommit {
            id: String,
        }

        #[derive(Deserialize)]
        struct LogResponse {
            results: Vec<SrhtCommit>,
        }

        let data: LogResponse = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let count = data
            .results
            .iter()
            .take_while(|c| !c.id.starts_with(&input.rev) && input.rev != c.id)
            .count();

        Ok(count)
    }

    async fn check_git_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let clone_url = get_clone_url(input);
        let cache_path = self.cache_path(&clone_url);
        let reference = input.reference.clone();
        let rev = input.rev.clone();
        let cancel = self.cancel_token.clone();

        debug!(input = %input.name, "Using git2 fallback");

        let result = tokio::time::timeout(
            self.timeouts.git_update_check,
            tokio::task::spawn_blocking(move || {
                if cancel.is_cancelled() {
                    return Err(GitError::CloneFailed("Cancelled".to_string()));
                }

                let repo = ensure_repo(&cache_path, &clone_url, reference.as_deref())?;
                let commits = get_commits_since(&repo, &rev, reference.as_deref())?;
                Ok(commits.len())
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(count))) => Ok(count),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(e)) => Err(GitError::CloneFailed(format!("Task failed: {}", e))),
            Err(_) => Err(GitError::NetworkError(
                "Timeout checking updates".to_string(),
            )),
        }
    }

    pub async fn get_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        debug!(input = %input.name, forge = ?input.forge_type, "Loading changelog");

        match input.forge_type {
            ForgeType::GitHub => self.get_github_changelog(input).await,
            ForgeType::GitLab => self.get_gitlab_changelog(input).await,
            ForgeType::SourceHut => self.get_sourcehut_changelog(input).await,
            _ => self.get_git_changelog(input).await,
        }
    }

    /// Get changelog via GitHub API
    async fn get_github_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let branch = input.reference.as_deref().unwrap_or("HEAD");

        // Get commits from branch
        let url = format!(
            "https://api.github.com/repos/{}/{}/commits?sha={}&per_page=100",
            input.owner, input.repo, branch
        );

        let mut req = self.client.get(&url);
        if let Some(token) = &self.github_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let status = resp.status();

        // Check for rate limiting
        if status.as_u16() == 403 || status.as_u16() == 429 {
            let remaining = resp
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);

            if remaining == 0 {
                return Err(GitError::NetworkError(
                    "GitHub API rate limit exceeded. Set GITHUB_TOKEN for higher limits."
                        .to_string(),
                ));
            }
        }

        if !status.is_success() {
            return self.get_git_changelog(input).await;
        }

        #[derive(Deserialize)]
        struct GitHubAuthor {
            name: Option<String>,
            date: Option<String>,
        }

        #[derive(Deserialize)]
        struct GitHubCommitData {
            message: String,
            author: Option<GitHubAuthor>,
        }

        #[derive(Deserialize)]
        struct GitHubCommit {
            sha: String,
            commit: GitHubCommitData,
        }

        let commits: Vec<GitHubCommit> = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let mut result_commits = Vec::new();
        let mut locked_idx = None;

        for (idx, c) in commits.iter().enumerate() {
            let is_locked = c.sha.starts_with(&input.rev) || c.sha == input.rev;
            if is_locked {
                locked_idx = Some(idx);
            }

            let date = c
                .commit
                .author
                .as_ref()
                .and_then(|a| a.date.as_ref())
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let author = c
                .commit
                .author
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let message = c.commit.message.lines().next().unwrap_or("").to_string();

            result_commits.push(Commit {
                sha: c.sha.clone(),
                message,
                author,
                date,
                is_locked,
            });
        }

        Ok(ChangelogData {
            commits: result_commits,
            locked_idx,
        })
    }

    /// Get changelog via GitLab API
    async fn get_gitlab_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let host = input.host.as_deref().unwrap_or("gitlab.com");
        let branch = input.reference.as_deref().unwrap_or("HEAD");

        let project = format!("{}/{}", input.owner, input.repo);
        let encoded_project = urlencoding(&project);

        let url = format!(
            "https://{}/api/v4/projects/{}/repository/commits?ref_name={}&per_page=100",
            host, encoded_project, branch
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return self.get_git_changelog(input).await;
        }

        #[derive(Deserialize)]
        struct GitLabCommit {
            id: String,
            title: String,
            author_name: String,
            created_at: String,
        }

        let commits: Vec<GitLabCommit> = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let mut result_commits = Vec::new();
        let mut locked_idx = None;

        for (idx, c) in commits.iter().enumerate() {
            let is_locked = c.id.starts_with(&input.rev) || c.id == input.rev;
            if is_locked {
                locked_idx = Some(idx);
            }

            let date = chrono::DateTime::parse_from_rfc3339(&c.created_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            result_commits.push(Commit {
                sha: c.id.clone(),
                message: c.title.clone(),
                author: c.author_name.clone(),
                date,
                is_locked,
            });
        }

        Ok(ChangelogData {
            commits: result_commits,
            locked_idx,
        })
    }

    /// Get changelog via SourceHut API
    async fn get_sourcehut_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let host = input.host.as_deref().unwrap_or("git.sr.ht");
        let owner = if input.owner.starts_with('~') {
            input.owner.clone()
        } else {
            format!("~{}", input.owner)
        };
        let branch = input.reference.as_deref().unwrap_or("HEAD");

        let url = format!(
            "https://{}/api/{}/{}/log/{}",
            host, owner, input.repo, branch
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return self.get_git_changelog(input).await;
        }

        #[derive(Deserialize)]
        struct SrhtAuthor {
            name: String,
        }

        #[derive(Deserialize)]
        struct SrhtCommit {
            id: String,
            message: String,
            author: SrhtAuthor,
            timestamp: String,
        }

        #[derive(Deserialize)]
        struct LogResponse {
            results: Vec<SrhtCommit>,
        }

        let data: LogResponse = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let mut result_commits = Vec::new();
        let mut locked_idx = None;

        for (idx, c) in data.results.iter().enumerate() {
            let is_locked = c.id.starts_with(&input.rev) || c.id == input.rev;
            if is_locked {
                locked_idx = Some(idx);
            }

            let date = chrono::DateTime::parse_from_rfc3339(&c.timestamp)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let message = c.message.lines().next().unwrap_or("").to_string();

            result_commits.push(Commit {
                sha: c.id.clone(),
                message,
                author: c.author.name.clone(),
                date,
                is_locked,
            });
        }

        Ok(ChangelogData {
            commits: result_commits,
            locked_idx,
        })
    }

    async fn get_git_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let clone_url = get_clone_url(input);
        let cache_path = self.cache_path(&clone_url);
        let reference = input.reference.clone();
        let rev = input.rev.clone();
        let cancel = self.cancel_token.clone();

        let result = tokio::time::timeout(
            self.timeouts.git_changelog,
            tokio::task::spawn_blocking(move || {
                if cancel.is_cancelled() {
                    return Err(GitError::CloneFailed("Cancelled".to_string()));
                }

                let repo = ensure_repo(&cache_path, &clone_url, reference.as_deref())?;

                let commits_ahead = get_commits_since(&repo, &rev, reference.as_deref())?;
                let commits_from_locked = get_commits_from(&repo, &rev, 50)?;

                let mut all_commits = commits_ahead;
                let locked_idx = if !commits_from_locked.is_empty() {
                    let idx = all_commits.len();
                    let mut locked_commits = commits_from_locked;
                    if let Some(first) = locked_commits.first_mut() {
                        first.is_locked = true;
                    }
                    all_commits.extend(locked_commits);
                    Some(idx)
                } else {
                    None
                };

                Ok(ChangelogData {
                    commits: all_commits,
                    locked_idx,
                })
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(data))) => Ok(data),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(e)) => Err(GitError::CloneFailed(format!("Task failed: {}", e))),
            Err(_) => Err(GitError::NetworkError(
                "Timeout loading changelog".to_string(),
            )),
        }
    }

    /// Get the cache path for a URL
    fn cache_path(&self, url: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();

        let safe_name: String = url
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(32)
            .collect();

        self.cache_dir.join(format!("{}_{:x}", safe_name, hash))
    }
}

/// Simple URL encoding for project paths
fn urlencoding(s: &str) -> String {
    s.replace('/', "%2F")
}

/// Get the XDG cache directory for melt
fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("melt")
        .join("git")
}

/// Get the clone URL for a git input
fn get_clone_url(input: &GitInput) -> String {
    input
        .forge_type
        .clone_url(&input.owner, &input.repo, input.host.as_deref())
}

/// Create git fetch options with SSH agent authentication
fn create_fetch_options<'a>() -> FetchOptions<'a> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            Cred::ssh_key_from_agent(username)
        } else if allowed_types.contains(git2::CredentialType::DEFAULT) {
            Cred::default()
        } else {
            Err(git2::Error::from_str("No supported credential type"))
        }
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    fetch_options
}

fn ensure_repo(
    cache_path: &Path,
    url: &str,
    reference: Option<&str>,
) -> Result<Repository, GitError> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GitError::CacheError(e.to_string()))?;
    }

    if cache_path.exists() {
        let repo = Repository::open_bare(cache_path)?;
        fetch_repo(&repo)?;
        Ok(repo)
    } else {
        clone_repo(cache_path, url, reference)
    }
}

fn clone_repo(
    cache_path: &Path,
    url: &str,
    reference: Option<&str>,
) -> Result<Repository, GitError> {
    debug!(url = %url, "Cloning repository");

    let mut builder = git2::build::RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(create_fetch_options());

    if let Some(r) = reference {
        builder.branch(r);
    }

    builder.clone(url, cache_path).map_err(GitError::from)
}

fn fetch_repo(repo: &Repository) -> Result<(), GitError> {
    let mut remote = repo.find_remote("origin")?;
    let refspecs: Vec<String> = remote
        .refspecs()
        .filter_map(|r| r.str().map(String::from))
        .collect();
    let refspec_strs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();

    remote.fetch(&refspec_strs, Some(&mut create_fetch_options()), None)?;
    Ok(())
}

/// Get commits since a given revision
fn get_commits_since(
    repo: &Repository,
    base_rev: &str,
    head_ref: Option<&str>,
) -> Result<Vec<Commit>, GitError> {
    let head_ref = head_ref.unwrap_or("HEAD");

    let head_oid = resolve_ref(repo, head_ref)?;

    let base_oid = match repo.revparse_single(base_rev) {
        Ok(obj) => obj.id(),
        Err(_) => return Ok(Vec::new()),
    };

    if head_oid == base_oid {
        return Ok(Vec::new());
    }

    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_oid)?;
    let _ = revwalk.hide(base_oid);

    let mut commits = Vec::new();
    for oid_result in revwalk.take(500) {
        let oid = oid_result?;
        if let Ok(commit) = repo.find_commit(oid) {
            commits.push(commit_to_model(&commit));
        }
    }

    Ok(commits)
}

/// Get commits starting from a revision going back
fn get_commits_from(repo: &Repository, rev: &str, limit: usize) -> Result<Vec<Commit>, GitError> {
    let oid = match repo.revparse_single(rev) {
        Ok(obj) => obj.id(),
        Err(_) => return Ok(Vec::new()),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.push(oid)?;

    let mut commits = Vec::new();
    for oid_result in revwalk.take(limit) {
        let oid = oid_result?;
        if let Ok(commit) = repo.find_commit(oid) {
            commits.push(commit_to_model(&commit));
        }
    }

    Ok(commits)
}

/// Resolve a reference to an OID
fn resolve_ref(repo: &Repository, refname: &str) -> Result<git2::Oid, GitError> {
    if let Ok(reference) = repo.find_reference(&format!("refs/remotes/origin/{}", refname)) {
        if let Some(oid) = reference.target() {
            return Ok(oid);
        }
    }

    if let Ok(reference) = repo.find_reference(&format!("refs/heads/{}", refname)) {
        if let Some(oid) = reference.target() {
            return Ok(oid);
        }
    }

    if refname == "HEAD" {
        if let Ok(head) = repo.head() {
            if let Some(oid) = head.target() {
                return Ok(oid);
            }
        }
    }

    if let Ok(obj) = repo.revparse_single(refname) {
        return Ok(obj.id());
    }

    Err(GitError::RevisionNotFound(refname.to_string()))
}

/// Convert a git2 commit to our Commit model
fn commit_to_model(commit: &git2::Commit) -> Commit {
    let sha = commit.id().to_string();
    let message = commit.summary().unwrap_or("").to_string();
    let author = commit.author().name().unwrap_or("Unknown").to_string();
    let time = commit.time();
    let date = Utc
        .timestamp_opt(time.seconds(), 0)
        .single()
        .unwrap_or_else(Utc::now);

    Commit {
        sha,
        message,
        author,
        date,
        is_locked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path() {
        let cancel = CancellationToken::new();
        let service = GitService::new(cancel);

        let path1 = service.cache_path("https://github.com/NixOS/nixpkgs.git");
        let path2 = service.cache_path("https://github.com/NixOS/nixpkgs.git");
        let path3 = service.cache_path("https://github.com/other/repo.git");

        assert_eq!(path1, path2);
        assert_ne!(path1, path3);
    }

    #[test]
    fn test_get_clone_url() {
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
    fn test_urlencoding() {
        assert_eq!(urlencoding("owner/repo"), "owner%2Frepo");
        assert_eq!(urlencoding("simple"), "simple");
    }
}
