use chrono::{DateTime, Utc};

/// A git commit.
#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: DateTime<Utc>,
}

impl Commit {
    /// Get the short SHA (first 7 characters)
    pub fn short_sha(&self) -> &str {
        &self.sha[..7.min(self.sha.len())]
    }
}

/// Valid commit index into a `ChangelogData` commit list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitIndex {
    index: usize,
}

impl CommitIndex {
    pub fn new(index: usize, len: usize) -> Option<Self> {
        (index < len).then_some(Self { index })
    }

    pub fn index(self) -> usize {
        self.index
    }
}

/// Error returned when changelog data cannot satisfy its invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangelogDataError {
    LockedIndexOutOfRange { index: usize, len: usize },
}

/// Result of fetching changelog for an input.
#[derive(Debug, Clone)]
pub struct ChangelogData {
    pub commits: Vec<Commit>,
    locked: Option<CommitIndex>,
}

impl ChangelogData {
    pub fn new(
        commits: Vec<Commit>,
        locked_idx: Option<usize>,
    ) -> Result<Self, ChangelogDataError> {
        let locked = match locked_idx {
            Some(index) => Some(CommitIndex::new(index, commits.len()).ok_or(
                ChangelogDataError::LockedIndexOutOfRange {
                    index,
                    len: commits.len(),
                },
            )?),
            None => None,
        };

        Ok(Self { commits, locked })
    }

    pub fn locked_index(&self) -> Option<usize> {
        self.locked.map(CommitIndex::index)
    }

    pub fn is_locked(&self, index: usize) -> bool {
        self.locked_index() == Some(index)
    }

    /// Get the number of new commits (ahead of locked)
    pub fn commits_ahead(&self) -> usize {
        self.locked_index().unwrap_or(self.commits.len())
    }

    /// Get the number of older commits (including and after locked)
    pub fn commits_behind(&self) -> usize {
        match self.locked_index() {
            Some(idx) => self.commits.len() - (idx + 1),
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
        };
        assert_eq!(commit.short_sha(), "abcdef1");

        let short = Commit {
            sha: "abc".to_string(),
            message: String::new(),
            author: String::new(),
            date: Utc::now(),
        };
        assert_eq!(short.short_sha(), "abc");
    }

    #[test]
    fn test_changelog_counts_when_locked_commit_is_missing() {
        let data = ChangelogData::new(commits(3), None).unwrap();

        assert_eq!(data.commits_ahead(), 3);
        assert_eq!(data.commits_behind(), 0);
    }

    #[test]
    fn test_changelog_counts_when_locked_commit_is_present() {
        let data = ChangelogData::new(commits(5), Some(3)).unwrap();

        assert_eq!(data.commits_ahead(), 3);
        assert_eq!(data.commits_behind(), 1);
        assert!(data.is_locked(3));
    }

    #[test]
    fn test_changelog_counts_at_edges() {
        let current = ChangelogData::new(commits(3), Some(0)).unwrap();
        assert_eq!(current.commits_ahead(), 0);
        assert_eq!(current.commits_behind(), 2);

        let empty = ChangelogData::new(Vec::new(), None).unwrap();
        assert_eq!(empty.commits_ahead(), 0);
        assert_eq!(empty.commits_behind(), 0);
    }

    #[test]
    fn test_changelog_rejects_out_of_range_locked_commit() {
        assert!(matches!(
            ChangelogData::new(commits(2), Some(5)),
            Err(ChangelogDataError::LockedIndexOutOfRange { index: 5, len: 2 })
        ));
    }
}
