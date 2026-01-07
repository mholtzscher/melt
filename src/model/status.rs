use std::time::Instant;

/// Status of update check for an input
#[derive(Debug, Clone, Default)]
pub enum UpdateStatus {
    /// Update status is not yet known
    #[default]
    Unknown,
    /// Currently checking for updates
    Checking,
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
    /// Create a new info message
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
            expires: Some(Instant::now() + std::time::Duration::from_secs(3)),
        }
    }

    /// Create a new error message
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Error,
            expires: Some(Instant::now() + std::time::Duration::from_secs(5)),
        }
    }

    /// Create a new warning message
    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Warning,
            expires: Some(Instant::now() + std::time::Duration::from_secs(4)),
        }
    }

    /// Check if the message has expired
    pub fn is_expired(&self) -> bool {
        self.expires.map(|e| Instant::now() > e).unwrap_or(false)
    }
}
