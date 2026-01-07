use chrono::{DateTime, Utc};

/// A git commit
#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: DateTime<Utc>,
    pub is_locked: bool,
}

impl Commit {
    /// Get the short SHA (first 7 characters)
    pub fn short_sha(&self) -> &str {
        &self.sha[..7.min(self.sha.len())]
    }
}

/// Result of fetching changelog for an input
#[derive(Debug, Clone)]
pub struct ChangelogData {
    pub commits: Vec<Commit>,
    /// Index of the currently locked commit, or None if not found
    pub locked_idx: Option<usize>,
}

impl ChangelogData {
    /// Get the number of new commits (ahead of locked)
    pub fn commits_ahead(&self) -> usize {
        self.locked_idx.unwrap_or(self.commits.len())
    }

    /// Get the number of older commits (including and after locked)
    pub fn commits_behind(&self) -> usize {
        match self.locked_idx {
            Some(idx) => self.commits.len().saturating_sub(idx + 1),
            None => 0,
        }
    }
}
