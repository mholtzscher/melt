use std::sync::Arc;
use std::time::{Duration, Instant};

use melt::app::ports::{ClockPort, GitPort, NixPort, SystemClock};
use melt::app::status::StatusMessage;
use melt::{GitService, NixService};
use tokio_util::sync::CancellationToken;

fn assert_nix_port_impl<T: NixPort>() {}
fn assert_git_port_impl<T: GitPort>() {}

#[test]
fn service_adapters_implement_port_contracts() {
    assert_nix_port_impl::<NixService>();
    assert_git_port_impl::<GitService>();
}

#[test]
fn services_can_be_used_as_port_trait_objects() {
    let cancel_token = CancellationToken::new();
    let nix: Arc<dyn NixPort> = Arc::new(NixService::new(cancel_token.clone()));
    let git: Arc<dyn GitPort> = Arc::new(GitService::new(cancel_token));

    assert!(Arc::strong_count(&nix) >= 1);
    assert!(Arc::strong_count(&git) >= 1);
}

#[test]
fn status_expiry_uses_injected_clock_time() {
    let now = Instant::now();
    let success = StatusMessage::success_at(now, "done");

    assert!(!success.is_expired_at(now + Duration::from_secs(2)));
    assert!(success.is_expired_at(now + Duration::from_secs(4)));
}

#[test]
fn system_clock_provides_monotonic_time_samples() {
    let clock = SystemClock;
    let t1 = clock.now();
    let t2 = clock.now();

    assert!(t2 >= t1);
}
