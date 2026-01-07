use std::path::PathBuf;
use thiserror::Error;

/// Application-level errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("No flake.nix found in {0}")]
    FlakeNotFound(PathBuf),

    #[error("Nix command failed: {0}")]
    NixCommandFailed(String),

    #[error("Failed to parse flake metadata: {0}")]
    MetadataParseError(String),

    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Git-specific errors
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Failed to clone repository: {0}")]
    CloneFailed(String),

    #[error("Repository not found")]
    NotFound,

    #[error("Authentication failed - ensure SSH agent is running with valid keys")]
    AuthFailed,

    #[error("Revision '{0}' not found in repository")]
    RevisionNotFound(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Cache directory error: {0}")]
    CacheError(String),
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        match e.code() {
            git2::ErrorCode::NotFound => GitError::NotFound,
            git2::ErrorCode::Auth => GitError::AuthFailed,
            git2::ErrorCode::GenericError if e.message().contains("not found") => {
                GitError::NotFound
            }
            git2::ErrorCode::GenericError if e.message().contains("resolve") => {
                GitError::NetworkError(e.message().to_string())
            }
            _ => GitError::CloneFailed(e.message().to_string()),
        }
    }
}

/// Result type alias for app operations
pub type AppResult<T> = Result<T, AppError>;
