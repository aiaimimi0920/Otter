//! 直接控制各链结果与取消的可见顺序，不靠调度竞争构造终态。
use super::*;
use live_relay_http::Error::{AuthorizationRequired, Status};

#[test]
fn source_exit_preserves_every_authorization_terminal_despite_other_failures_and_stop() {
    let control_failure = Some(Err(control::Error::Panicked));
    let observation_failure = Some(Err(observation::Error::Source));
    let denied = Exit::Http(AuthorizationRequired);
    for terminal in [denied, Exit::Revoked] {
        assert_eq!(
            source_exit(true, Err(terminal), control_failure, observation_failure),
            terminal
        );
        assert!(terminal.authorization_ended());
    }
    let control = control::Error::Http(AuthorizationRequired);
    assert_eq!(
        source_exit(
            true,
            Err(Exit::Budget),
            Some(Err(control)),
            observation_failure
        ),
        Exit::Control(control)
    );
    let observation = observation::Error::Http(AuthorizationRequired);
    assert_eq!(
        source_exit(
            true,
            Err(Exit::Budget),
            control_failure,
            Some(Err(observation))
        ),
        Exit::Observation(observation)
    );
    assert_eq!(
        source_exit(true, Err(Exit::Http(Status(503))), None, None),
        Exit::Stopped
    );
    assert_eq!(
        source_exit(
            true,
            Err(Exit::Budget),
            control_failure,
            observation_failure
        ),
        Exit::Control(control::Error::Panicked)
    );
    assert!(!Exit::Stopped.authorization_ended());
}

#[test]
fn source_progress_preserves_received_denial_but_not_late_success_or_transient_failure() {
    let stopped = || Err(Exit::Stopped);
    assert_eq!(
        recovery::progress_after_check(Err(AuthorizationRequired), stopped),
        Err(Exit::Http(AuthorizationRequired))
    );
    assert_eq!(
        recovery::progress_after_check(Ok(Some(4)), stopped),
        Err(Exit::Stopped)
    );
    assert_eq!(
        recovery::progress_after_check(Err(Status(503)), stopped),
        Err(Exit::Stopped)
    );
    assert_eq!(
        recovery::progress_after_check(Ok(Some(4)), || Ok(())),
        Ok(Some(4))
    );
}
