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
            nix_command: Duration::from_secs(30),
            git_update_check: Duration::from_secs(60),
            git_changelog: Duration::from_secs(120),
            http_request: Duration::from_secs(30),
        }
    }
}

impl Timeouts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_nix_command(mut self, timeout: Duration) -> Self {
        self.nix_command = timeout;
        self
    }

    pub fn with_git_update_check(mut self, timeout: Duration) -> Self {
        self.git_update_check = timeout;
        self
    }

    pub fn with_git_changelog(mut self, timeout: Duration) -> Self {
        self.git_changelog = timeout;
        self
    }

    pub fn with_http_request(mut self, timeout: Duration) -> Self {
        self.http_request = timeout;
        self
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

impl ServiceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub fn with_git_concurrency(mut self, concurrency: usize) -> Self {
        self.git_concurrency = concurrency;
        self
    }
}
