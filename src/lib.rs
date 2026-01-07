//! melt - A TUI for managing Nix flake inputs
//!
//! This library provides the core functionality for the melt TUI application,
//! which helps users manage Nix flake inputs with features like:
//!
//! - Viewing all flake inputs with their current revision and update status
//! - Checking for available updates across multiple git forges
//! - Updating individual or multiple inputs
//! - Viewing changelogs and locking to specific commits
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`app`]: Application core with state management and event handling
//! - [`error`]: Error types for the application
//! - [`model`]: Domain models for flakes, inputs, commits, etc.
//! - [`service`]: Services for interacting with Nix and Git
//! - [`ui`]: UI rendering and theming
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//! use melt::{App, Tui};
//!
//! #[tokio::main]
//! async fn main() -> melt::AppResult<()> {
//!     let mut tui = Tui::new()?;
//!     let mut app = App::new(PathBuf::from("."));
//!     app.run(&mut tui).await
//! }
//! ```

pub mod app;
pub mod config;
pub mod error;
pub mod event;
pub mod model;
pub mod service;
pub mod tui;
pub mod ui;
pub mod util;

// Re-export commonly used types at the crate root
pub use app::App;
pub use config::{ServiceConfig, Timeouts};
pub use error::{AppError, AppResult, GitError};
pub use model::{
    ChangelogData, Commit, FlakeData, FlakeInput, ForgeType, GitInput, OtherInput, PathInput,
    StatusLevel, StatusMessage, UpdateStatus,
};
pub use service::{GitOperations, GitService, NixOperations, NixService};
pub use tui::Tui;
