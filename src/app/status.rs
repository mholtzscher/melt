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
    /// Create a new info message.
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Info,
            expires: None,
        }
    }

    /// Create a success message with explicit current time.
    pub fn success_at(now: Instant, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Success,
            expires: Some(now + Duration::from_secs(3)),
        }
    }

    /// Create an error message with explicit current time.
    pub fn error_at(now: Instant, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Error,
            expires: Some(now + Duration::from_secs(5)),
        }
    }

    /// Create a warning message with explicit current time.
    pub fn warning_at(now: Instant, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: StatusLevel::Warning,
            expires: Some(now + Duration::from_secs(4)),
        }
    }

    /// Check if the message has expired at a given time.
    pub fn is_expired_at(&self, now: Instant) -> bool {
        self.expires.map(|e| now > e).unwrap_or(false)
    }
}
