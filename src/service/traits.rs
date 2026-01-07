use std::path::Path;

use crate::error::{AppResult, GitError};
use crate::model::{ChangelogData, FlakeData, FlakeInput, GitInput, UpdateStatus};

pub trait NixOperations: Clone + Send + Sync {
    fn load_metadata(
        &self,
        path: &Path,
    ) -> impl std::future::Future<Output = AppResult<FlakeData>> + Send;

    fn update_inputs(
        &self,
        path: &Path,
        names: &[String],
    ) -> impl std::future::Future<Output = AppResult<()>> + Send;

    fn update_all(&self, path: &Path) -> impl std::future::Future<Output = AppResult<()>> + Send;

    fn lock_input(
        &self,
        path: &Path,
        name: &str,
        override_url: &str,
    ) -> impl std::future::Future<Output = AppResult<()>> + Send;
}

pub trait GitOperations: Clone + Send + Sync {
    fn check_updates<F>(
        &self,
        inputs: &[FlakeInput],
        on_status: F,
    ) -> impl std::future::Future<Output = Result<(), GitError>> + Send
    where
        F: FnMut(&str, UpdateStatus) + Send;

    fn get_changelog(
        &self,
        input: &GitInput,
    ) -> impl std::future::Future<Output = Result<ChangelogData, GitError>> + Send;
}
