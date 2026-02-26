use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Instant;

use crate::error::{AppResult, GitError};
use crate::model::{ChangelogData, FlakeData, GitInput, UpdateStatus};

pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type StatusCallback<'a> = Box<dyn FnMut(&str, UpdateStatus) + Send + 'a>;

pub trait NixPort: Send + Sync {
    fn load_metadata<'a>(&'a self, path: &'a Path) -> PortFuture<'a, AppResult<FlakeData>>;
    fn update_inputs<'a>(
        &'a self,
        path: &'a Path,
        names: &'a [String],
    ) -> PortFuture<'a, AppResult<()>>;
    fn update_all<'a>(&'a self, path: &'a Path) -> PortFuture<'a, AppResult<()>>;
    fn lock_input<'a>(
        &'a self,
        path: &'a Path,
        name: &'a str,
        override_url: &'a str,
    ) -> PortFuture<'a, AppResult<()>>;
}

pub trait GitPort: Send + Sync {
    fn get_changelog<'a>(
        &'a self,
        input: &'a GitInput,
    ) -> PortFuture<'a, Result<ChangelogData, GitError>>;
    fn check_updates<'a>(
        &'a self,
        inputs: &'a [GitInput],
        on_status: StatusCallback<'a>,
    ) -> PortFuture<'a, Result<(), GitError>>;
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
