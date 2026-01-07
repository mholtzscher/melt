use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::model::{ChangelogData, Commit, FlakeInput, GitInput, UpdateStatus};
#[cfg(test)]
use crate::model::ForgeType;

/// Service for git operations using git2
#[derive(Clone)]
pub struct GitService {
    cache_dir: PathBuf,
    cancel_token: CancellationToken,
    /// Semaphore to limit concurrent git operations
    semaphore: Arc<Semaphore>,
}

impl GitService {
    /// Create a new GitService
    pub fn new(cancel_token: CancellationToken) -> Self {
        let cache_dir = get_cache_dir();
        Self {
            cache_dir,
            cancel_token,
            semaphore: Arc::new(Semaphore::new(10)), // Max 10 concurrent operations
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
        // Filter to only git inputs
        let git_inputs: Vec<&GitInput> = inputs
            .iter()
            .filter_map(|i| match i {
                FlakeInput::Git(g) => Some(g),
                _ => None,
            })
            .collect();

        // Set all to checking
        for input in &git_inputs {
            on_status(&input.name, UpdateStatus::Checking);
        }

        // Check each input
        for input in git_inputs {
            if self.cancel_token.is_cancelled() {
                break;
            }

            let _permit = self.semaphore.acquire().await.map_err(|_| {
                GitError::CloneFailed("Failed to acquire semaphore".to_string())
            })?;

            let status = match self.check_input_updates(input).await {
                Ok(count) => {
                    if count == 0 {
                        UpdateStatus::UpToDate
                    } else {
                        UpdateStatus::Behind(count)
                    }
                }
                Err(e) => UpdateStatus::Error(e.to_string()),
            };

            on_status(&input.name, status);
        }

        Ok(())
    }

    /// Check for updates on a single input
    async fn check_input_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let clone_url = get_clone_url(input);
        let cache_path = self.cache_path(&clone_url);
        let reference = input.reference.clone();
        let rev = input.rev.clone();
        let cancel = self.cancel_token.clone();

        // Run git operations in blocking task
        tokio::task::spawn_blocking(move || {
            if cancel.is_cancelled() {
                return Err(GitError::CloneFailed("Cancelled".to_string()));
            }

            let repo = ensure_repo(&cache_path, &clone_url, reference.as_deref())?;
            let commits = get_commits_since(&repo, &rev, reference.as_deref())?;
            Ok(commits.len())
        })
        .await
        .map_err(|e| GitError::CloneFailed(e.to_string()))?
    }

    /// Get changelog for an input
    pub async fn get_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let clone_url = get_clone_url(input);
        let cache_path = self.cache_path(&clone_url);
        let reference = input.reference.clone();
        let rev = input.rev.clone();
        let cancel = self.cancel_token.clone();

        tokio::task::spawn_blocking(move || {
            if cancel.is_cancelled() {
                return Err(GitError::CloneFailed("Cancelled".to_string()));
            }

            let repo = ensure_repo(&cache_path, &clone_url, reference.as_deref())?;
            
            // Get commits ahead of locked rev
            let commits_ahead = get_commits_since(&repo, &rev, reference.as_deref())?;
            
            // Get commits from locked rev going back
            let commits_from_locked = get_commits_from(&repo, &rev, 50)?;

            // Combine: ahead commits + locked + older commits
            let mut all_commits = commits_ahead;
            let locked_idx = if !commits_from_locked.is_empty() {
                let idx = all_commits.len();
                // Mark first commit as locked
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
        })
        .await
        .map_err(|e| GitError::CloneFailed(e.to_string()))?
    }

    /// Get the cache path for a URL
    fn cache_path(&self, url: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();

        // Create a safe filename from the URL
        let safe_name: String = url
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(32)
            .collect();

        self.cache_dir.join(format!("{}_{:x}", safe_name, hash))
    }
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
    input.forge_type.clone_url(&input.owner, &input.repo, input.host.as_deref())
}

/// Create git fetch options with SSH agent authentication
fn create_fetch_options<'a>() -> FetchOptions<'a> {
    let mut callbacks = RemoteCallbacks::new();
    
    callbacks.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            // Try SSH agent first
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

/// Ensure a bare repository exists in cache, cloning or fetching as needed
fn ensure_repo(cache_path: &Path, url: &str, reference: Option<&str>) -> Result<Repository, GitError> {
    // Create parent directory if needed
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GitError::CacheError(e.to_string()))?;
    }

    if cache_path.exists() {
        // Open existing repo and fetch
        let repo = Repository::open_bare(cache_path)?;
        fetch_repo(&repo)?;
        Ok(repo)
    } else {
        // Clone new bare repo
        clone_repo(cache_path, url, reference)
    }
}

/// Clone a bare repository
fn clone_repo(cache_path: &Path, url: &str, reference: Option<&str>) -> Result<Repository, GitError> {
    let mut builder = git2::build::RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(create_fetch_options());

    // If reference is specified, fetch only that branch
    if let Some(r) = reference {
        builder.branch(r);
    }

    builder.clone(url, cache_path).map_err(GitError::from)
}

/// Fetch updates for an existing repository
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
    
    // Resolve the head reference
    let head_oid = resolve_ref(repo, head_ref)?;
    
    // Try to resolve the base revision
    let base_oid = match repo.revparse_single(base_rev) {
        Ok(obj) => obj.id(),
        Err(_) => return Ok(Vec::new()), // Base not found, return empty
    };

    // If they're the same, no commits to return
    if head_oid == base_oid {
        return Ok(Vec::new());
    }

    // Walk commits from head
    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_oid)?;
    
    // Hide everything reachable from base
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
    // Try as a remote ref first (origin/<branch>)
    if let Ok(reference) = repo.find_reference(&format!("refs/remotes/origin/{}", refname)) {
        if let Some(oid) = reference.target() {
            return Ok(oid);
        }
    }

    // Try as a local ref
    if let Ok(reference) = repo.find_reference(&format!("refs/heads/{}", refname)) {
        if let Some(oid) = reference.target() {
            return Ok(oid);
        }
    }

    // Try as HEAD
    if refname == "HEAD" {
        if let Ok(head) = repo.head() {
            if let Some(oid) = head.target() {
                return Ok(oid);
            }
        }
    }

    // Try as a direct revision
    if let Ok(obj) = repo.revparse_single(refname) {
        return Ok(obj.id());
    }

    Err(GitError::RevisionNotFound(refname.to_string()))
}

/// Convert a git2 commit to our Commit model
fn commit_to_model(commit: &git2::Commit) -> Commit {
    let sha = commit.id().to_string();
    let message = commit
        .summary()
        .unwrap_or("")
        .to_string();
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
}
