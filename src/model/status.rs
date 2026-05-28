use std::time::{Duration, Instant};

/// Status of update check for an input
#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    /// Update status is not yet known
    #[default]
    Unknown,
    /// Currently checking for updates
    Checking,
    /// Currently being updated
    Updating,
    /// Input is up to date with remote
    UpToDate,
    /// Input is behind remote by N commits
    Behind(usize),
    /// Error occurred while checking
    Error(String),
}

impl UpdateStatus {
    /// Get display string for the status
    pub fn display(&self) -> String {
        match self {
            UpdateStatus::Unknown => "-".to_string(),
            UpdateStatus::Checking => "...".to_string(),
            UpdateStatus::Updating => "...".to_string(),
            UpdateStatus::UpToDate => "ok".to_string(),
            UpdateStatus::Behind(n) => format!("+{}", n),
            UpdateStatus::Error(_) => "?".to_string(),
        }
    }
}

/// A status message to show in the status bar
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub expires: Option<Instant>,
}

/// Level of status message (affects styling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl StatusMessage {
    /// Create a new info message that does not expire automatically
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Info,
            expires: None,
        }
    }

    /// Create a new success message that expires
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Success,
            expires: Some(Instant::now() + Duration::from_secs(3)),
        }
    }

    /// Create a new error message
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Error,
            expires: Some(Instant::now() + Duration::from_secs(5)),
        }
    }

    /// Create a new warning message
    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Warning,
            expires: Some(Instant::now() + Duration::from_secs(4)),
        }
    }

    /// Check if the message has expired
    pub fn is_expired(&self) -> bool {
        self.expires.map(|e| Instant::now() > e).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_status_display() {
        assert_eq!(UpdateStatus::Unknown.display(), "-");
        assert_eq!(UpdateStatus::Checking.display(), "...");
        assert_eq!(UpdateStatus::Updating.display(), "...");
        assert_eq!(UpdateStatus::UpToDate.display(), "ok");
        assert_eq!(UpdateStatus::Behind(12).display(), "+12");
        assert_eq!(UpdateStatus::Error("failed".to_string()).display(), "?");
    }

    #[test]
    fn test_status_message_constructors() {
        let info = StatusMessage::info("loading");
        assert_eq!(info.text, "loading");
        assert_eq!(info.level, StatusLevel::Info);
        assert!(info.expires.is_none());
        assert!(!info.is_expired());

        let success = StatusMessage::success("done");
        assert_eq!(success.level, StatusLevel::Success);
        assert!(success.expires.is_some());

        let warning = StatusMessage::warning("careful");
        assert_eq!(warning.level, StatusLevel::Warning);
        assert!(warning.expires.is_some());

        let error = StatusMessage::error("failed");
        assert_eq!(error.level, StatusLevel::Error);
        assert!(error.expires.is_some());
    }

    #[test]
    fn test_status_message_expiry() {
        let expired = StatusMessage {
            text: "old".to_string(),
            level: StatusLevel::Info,
            expires: Some(Instant::now() - Duration::from_secs(1)),
        };
        assert!(expired.is_expired());

        let active = StatusMessage {
            text: "new".to_string(),
            level: StatusLevel::Info,
            expires: Some(Instant::now() + Duration::from_secs(1)),
        };
        assert!(!active.is_expired());
    }
}
