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
