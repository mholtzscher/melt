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

#[cfg(test)]
mod tests {
    use super::*;

    fn commits(count: usize) -> Vec<Commit> {
        (0..count)
            .map(|idx| Commit {
                sha: format!("abcdef{}", idx),
                message: "message".to_string(),
                author: "author".to_string(),
                date: Utc::now(),
                is_locked: false,
            })
            .collect()
    }

    #[test]
    fn test_short_sha() {
        let commit = Commit {
            sha: "abcdef123456".to_string(),
            message: String::new(),
            author: String::new(),
            date: Utc::now(),
            is_locked: false,
        };
        assert_eq!(commit.short_sha(), "abcdef1");

        let short = Commit {
            sha: "abc".to_string(),
            message: String::new(),
            author: String::new(),
            date: Utc::now(),
            is_locked: false,
        };
        assert_eq!(short.short_sha(), "abc");
    }

    #[test]
    fn test_changelog_counts_when_locked_commit_is_missing() {
        let data = ChangelogData {
            commits: commits(3),
            locked_idx: None,
        };

        assert_eq!(data.commits_ahead(), 3);
        assert_eq!(data.commits_behind(), 0);
    }

    #[test]
    fn test_changelog_counts_when_locked_commit_is_present() {
        let data = ChangelogData {
            commits: commits(5),
            locked_idx: Some(3),
        };

        assert_eq!(data.commits_ahead(), 3);
        assert_eq!(data.commits_behind(), 1);
    }

    #[test]
    fn test_changelog_counts_at_edges() {
        let current = ChangelogData {
            commits: commits(3),
            locked_idx: Some(0),
        };
        assert_eq!(current.commits_ahead(), 0);
        assert_eq!(current.commits_behind(), 2);

        let empty = ChangelogData {
            commits: Vec::new(),
            locked_idx: None,
        };
        assert_eq!(empty.commits_ahead(), 0);
        assert_eq!(empty.commits_behind(), 0);

        let out_of_range = ChangelogData {
            commits: commits(2),
            locked_idx: Some(5),
        };
        assert_eq!(out_of_range.commits_ahead(), 5);
        assert_eq!(out_of_range.commits_behind(), 0);
    }
}
