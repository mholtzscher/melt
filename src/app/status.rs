use std::time::{Duration, Instant};

/// Level of status message (affects styling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A status message to show in the status bar
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub expires: Option<Instant>,
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
