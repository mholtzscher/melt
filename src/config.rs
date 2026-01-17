use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Timeouts {
    pub nix_command: Duration,
    pub git_update_check: Duration,
    pub git_changelog: Duration,
    pub http_request: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            nix_command: Duration::from_secs(120),
            git_update_check: Duration::from_secs(120),
            git_changelog: Duration::from_secs(120),
            http_request: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub timeouts: Timeouts,
    pub git_concurrency: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            timeouts: Timeouts::default(),
            git_concurrency: 10,
        }
    }
}
