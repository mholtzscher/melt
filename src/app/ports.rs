use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use crate::error::{AppResult, GitError};
use crate::model::{ChangelogData, FlakeData, GitInput, UpdateStatus};

pub type PortFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type StatusCallback = Box<dyn FnMut(&str, UpdateStatus) + Send + 'static>;

pub trait NixPort: Send + Sync {
    fn load_metadata(self: Arc<Self>, path: std::path::PathBuf)
        -> PortFuture<AppResult<FlakeData>>;
    fn update_inputs(
        self: Arc<Self>,
        path: std::path::PathBuf,
        names: Vec<String>,
    ) -> PortFuture<AppResult<()>>;
    fn update_all(self: Arc<Self>, path: std::path::PathBuf) -> PortFuture<AppResult<()>>;
    fn lock_input(
        self: Arc<Self>,
        path: std::path::PathBuf,
        name: String,
        override_url: String,
    ) -> PortFuture<AppResult<()>>;
}

pub trait GitPort: Send + Sync {
    fn get_changelog(
        self: Arc<Self>,
        input: GitInput,
    ) -> PortFuture<Result<ChangelogData, GitError>>;
    fn check_updates(
        self: Arc<Self>,
        inputs: Vec<GitInput>,
        on_status: StatusCallback,
    ) -> PortFuture<Result<(), GitError>>;
}

pub trait ClockPort: Send + Sync {
    fn now(&self) -> Instant;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl ClockPort for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
