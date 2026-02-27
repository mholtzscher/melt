use std::path::{Path, PathBuf};
use chrono::{TimeZone, Utc};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use tokio_util::sync::CancellationToken;
use tracing::debug;
use std::time::Duration;

use crate::error::GitError;
use crate::model::{ChangelogData, Commit, ForgeType, GitInput};
use crate::policy::build_clone_url;

pub struct Git2Client<'a> {
    pub cache_dir: &'a Path,
    pub cancel_token: &'a CancellationToken,
    pub timeout: Duration,
}

impl<'a> Git2Client<'a> {
    pub async fn check_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let clone_url = ensure_clone_url(input)?;
        let cache_path = self.cache_path(&clone_url);
        let reference = input.reference.clone();
        let rev = input.rev.clone();
        let cancel = self.cancel_token.clone();

        debug!(input = %input.name, "Using git2 fallback");

        let result = tokio::time::timeout(
            self.timeout,
            tokio::task::spawn_blocking(move || {
                if cancel.is_cancelled() {
                    return Err(GitError::CloneFailed("Cancelled".to_string()));
                }

                let repo = ensure_repo(&cache_path, &clone_url, reference.as_deref(), &cancel)?;
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
        let clone_url = ensure_clone_url(input)?;
        let cache_path = self.cache_path(&clone_url);
        let reference = input.reference.clone();
        let rev = input.rev.clone();
        let cancel = self.cancel_token.clone();

        let result = tokio::time::timeout(
            self.timeout,
            tokio::task::spawn_blocking(move || {
                if cancel.is_cancelled() {
                    return Err(GitError::CloneFailed("Cancelled".to_string()));
                }

                let repo = ensure_repo(&cache_path, &clone_url, reference.as_deref(), &cancel)?;

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

/// Get the clone URL for a git input
pub fn get_clone_url(input: &GitInput) -> String {
    if input.forge_type == ForgeType::Generic {
        let url = input.url.trim();
        return url.strip_prefix("git+").unwrap_or(url).to_string();
    }

    build_clone_url(
        input.forge_type,
        &input.owner,
        &input.repo,
        input.host.as_deref(),
    )
    .unwrap_or_default()
}

fn ensure_clone_url(input: &GitInput) -> Result<String, GitError> {
    let url = get_clone_url(input);
    if url.trim().is_empty() {
        return Err(GitError::CloneFailed(format!(
            "Missing clone URL for input '{}'",
            input.name
        )));
    }

    Ok(url)
}

/// Create git fetch options with SSH agent authentication
fn create_fetch_options(cancel: &CancellationToken) -> FetchOptions<'_> {
    let cancel_for_progress = cancel.clone();

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

    callbacks.transfer_progress(move |_stats| !cancel_for_progress.is_cancelled());

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    fetch_options
}

fn ensure_repo(
    cache_path: &Path,
    url: &str,
    reference: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Repository, GitError> {
    if cancel.is_cancelled() {
        return Err(GitError::CloneFailed("Cancelled".to_string()));
    }

    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GitError::CacheError(e.to_string()))?;
    }

    if cache_path.exists() {
        let repo = Repository::open_bare(cache_path)?;
        fetch_repo(&repo, cancel)?;
        Ok(repo)
    } else {
        clone_repo(cache_path, url, reference, cancel)
    }
}

fn clone_repo(
    cache_path: &Path,
    url: &str,
    reference: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Repository, GitError> {
    debug!(url = %url, "Cloning repository");

    let mut builder = git2::build::RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(create_fetch_options(cancel));

    if let Some(r) = reference {
        builder.branch(r);
    }

    builder.clone(url, cache_path).map_err(GitError::from)
}

fn fetch_repo(repo: &Repository, cancel: &CancellationToken) -> Result<(), GitError> {
    let mut remote = repo.find_remote("origin")?;
    let refspecs: Vec<String> = remote
        .refspecs()
        .filter_map(|r| r.str().map(String::from))
        .collect();
    let refspec_strs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();

    let mut fetch_options = create_fetch_options(cancel);
    remote.fetch(&refspec_strs, Some(&mut fetch_options), None)?;
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

        for candidate in ["main", "master"] {
            if let Ok(reference) =
                repo.find_reference(&format!("refs/remotes/origin/{}", candidate))
            {
                if let Some(oid) = reference.target() {
                    return Ok(oid);
                }
            }

            if let Ok(reference) = repo.find_reference(&format!("refs/heads/{}", candidate)) {
                if let Some(oid) = reference.target() {
                    return Ok(oid);
                }
            }
        }

        if let Ok(references) = repo.references() {
            let mut single_remote = None;
            let mut remote_count = 0usize;

            for reference in references.flatten() {
                if let Some(name) = reference.name() {
                    if name.starts_with("refs/remotes/origin/") && !name.ends_with("/HEAD") {
                        remote_count += 1;
                        if remote_count == 1 {
                            single_remote = Some(name.to_string());
                        } else {
                            single_remote = None;
                            break;
                        }
                    }
                }
            }

            if remote_count == 1 {
                if let Some(reference_name) = single_remote {
                    if let Ok(reference) = repo.find_reference(&reference_name) {
                        if let Some(oid) = reference.target() {
                            return Ok(oid);
                        }
                    }
                }
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
