use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use crate::error::{AppResult, GitError};
use crate::model::{ChangelogData, FlakeData, FlakeInput, GitInput, UpdateStatus};
use crate::service::{GitService, NixService};

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
        inputs: &'a [FlakeInput],
        on_status: StatusCallback<'a>,
    ) -> PortFuture<'a, Result<(), GitError>>;
}

impl NixPort for NixService {
    fn load_metadata<'a>(&'a self, path: &'a Path) -> PortFuture<'a, AppResult<FlakeData>> {
        Box::pin(async move { NixService::load_metadata(self, path).await })
    }

    fn update_inputs<'a>(
        &'a self,
        path: &'a Path,
        names: &'a [String],
    ) -> PortFuture<'a, AppResult<()>> {
        Box::pin(async move { NixService::update_inputs(self, path, names).await })
    }

    fn update_all<'a>(&'a self, path: &'a Path) -> PortFuture<'a, AppResult<()>> {
        Box::pin(async move { NixService::update_all(self, path).await })
    }

    fn lock_input<'a>(
        &'a self,
        path: &'a Path,
        name: &'a str,
        override_url: &'a str,
    ) -> PortFuture<'a, AppResult<()>> {
        Box::pin(async move { NixService::lock_input(self, path, name, override_url).await })
    }
}

impl GitPort for GitService {
    fn get_changelog<'a>(
        &'a self,
        input: &'a GitInput,
    ) -> PortFuture<'a, Result<ChangelogData, GitError>> {
        Box::pin(async move { GitService::get_changelog(self, input).await })
    }

    fn check_updates<'a>(
        &'a self,
        inputs: &'a [FlakeInput],
        mut on_status: StatusCallback<'a>,
    ) -> PortFuture<'a, Result<(), GitError>> {
        Box::pin(async move {
            GitService::check_updates(self, inputs, move |name, status| on_status(name, status))
                .await
        })
    }
}
