use super::*;
use crate::{
    device_authorization_lease::Grant, device_identity::Identity, device_session_attempt::Attempt,
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForMultipleObjects,
};
#[path = "composition_relay_attach_tests.rs"]
mod attach;
#[path = "composition_relay_surface_events_tests.rs"]
mod surface_events;
#[path = "composition_relay_surface_resources_tests.rs"]
mod surface_resources;

fn surface_owner() -> crate::surface_lifecycle_owner::Owner {
    let binding=crate::surface_live_binding::Binding::from_mount(
        br#"{"instance":{"descriptor":{"instanceId":"surface:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"surface:test","deviceId":"viewer:test","hookNodeId":"node:test"},"lifecycle":"mounted","lifecycleRevision":1}}}}"#,
        "https://localhost:9","viewer:test","node:test").unwrap();
    crate::surface_lifecycle_owner::Owner::new(binding)
}
fn surface_config() -> Config {
    Config::new(
        "https://localhost:9",
        std::path::Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-relay-profile"),
        "test",
    )
    .unwrap()
}

#[test]
fn cancellation_discards_cached_surface_success_without_losing_owner() {
    let mut sessions = Sessions::default();
    sessions
        .surface_outcomes
        .push(crate::surface_lifecycle_runtime::Outcome {
            owner: surface_owner(),
            result: Ok(None),
        });
    sessions.cancel_surface_requests();
    let outcome = sessions.take_surface_outcome().unwrap();
    assert!(matches!(
        outcome.result,
        Err(crate::surface_lifecycle_runtime::Error::Stopped)
    ));
    assert_eq!(outcome.owner.binding().lifecycle_revision(), 1);
    assert!(!outcome.owner.uncertain());
}
#[test]
fn unmatched_lifecycle_delivery_restores_owner_and_capacity() {
    let mut sessions = Sessions::default();
    sessions
        .surface_outcomes
        .push(crate::surface_lifecycle_runtime::Outcome {
            owner: surface_owner(),
            result: Ok(None),
        });
    assert!(!sessions.deliver_surface(|outcome| Err(Box::new(outcome))));
    assert_eq!(sessions.occupied(), 1);
    assert!(sessions.deliver_surface(|outcome| {
        assert_eq!(outcome.owner.binding().attachment(), "attachment:test");
        assert!(outcome.result.is_ok());
        Ok(())
    }));
    assert_eq!(sessions.occupied(), 0);
    sessions
        .surface_outcomes
        .push(crate::surface_lifecycle_runtime::Outcome {
            owner: surface_owner(),
            result: Ok(None),
        });
    sessions
        .surface_outcomes
        .push(crate::surface_lifecycle_runtime::Outcome {
            owner: surface_owner(),
            result: Err(crate::surface_lifecycle_runtime::Error::Stopped),
        });
    assert!(sessions.deliver_surface(|outcome| {
        if outcome.result.is_ok() {
            Ok(())
        } else {
            Err(Box::new(outcome))
        }
    }));
    assert_eq!(sessions.occupied(), 1);
    assert!(sessions.take_surface_outcome().unwrap().result.is_err());
}
#[test]
fn surface_outcomes_keep_capacity_and_owner_until_explicitly_taken() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        sessions
            .start_surface(
                surface_config(),
                revoked(),
                surface_owner(),
                crate::surface_lifecycle_runtime::Action::Recover,
            )
            .unwrap_or_else(|e| panic!("{}", e.error));
    }
    while !sessions.finished() {
        assert!(sessions.poll_surfaces().is_empty());
        if sessions.finished() {
            break;
        }
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
    }
    assert_eq!(sessions.wait_handles().count(), 0);
    assert!(matches!(start(&mut sessions),Err(e) if e.kind()==io::ErrorKind::WouldBlock));
    assert!(
        matches!(sessions.discover(surface_config(),revoked()),Err(e) if e.kind()==io::ErrorKind::WouldBlock)
    );
    assert!(
        matches!(sessions.query_projection_inbox(surface_config(), revoked()),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
    sessions.close();
    for _ in 0..MAX_SESSIONS {
        let outcome = sessions.take_surface_outcome().unwrap();
        assert_eq!(outcome.owner.binding().instance(), "surface:test");
        assert!(matches!(
            outcome.result,
            Err(crate::surface_lifecycle_runtime::Error::Stopped)
        ));
    }
    assert!(sessions.take_surface_outcome().is_none());
    let Err(error) = sessions.start_surface(
        surface_config(),
        revoked(),
        surface_owner(),
        crate::surface_lifecycle_runtime::Action::Recover,
    ) else {
        panic!("closed collection accepted task")
    };
    assert_eq!(error.error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.owner.binding().instance(), "surface:test");
}

fn revoked() -> Lease {
    let mut identity = Identity::generate().unwrap();
    identity.bind("viewer:test").unwrap();
    let session = Attempt::new(&identity, "https://localhost:9")
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"viewer:test","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            br#"{"deviceId":"viewer:test","token":"secret","expiresAtMs":99999}"#,
            0,
        )
        .unwrap();
    let mut grant = Grant::new(session);
    grant.lease("https://localhost:9", 0).unwrap()
}
fn start(sessions: &mut Sessions) -> io::Result<()> {
    sessions.start(
        Config::new(
            "https://localhost:9",
            std::path::Path::new(
                "C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-relay-profile",
            ),
            "test",
        )
        .unwrap(),
        revoked(),
        Target {
            session: "live:test".into(),
            surface: "surface:test".into(),
            attachment: "attach:test".into(),
            expected: None,
            selected: None,
        },
        Budget::new(1024),
        |_| panic!("revoked callback"),
    )
}
#[test]
fn collection_bounds_unreaped_sessions_and_shutdown_leaves_no_handles() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        start(&mut sessions).unwrap();
    }
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS * 2);
    assert!(
        matches!(start(&mut sessions), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
    );
    sessions.close();
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    assert!(
        matches!(start(&mut sessions), Err(error) if error.kind() == io::ErrorKind::BrokenPipe)
    );
    let mut outcomes = Vec::new();
    while !sessions.finished() {
        outcomes.extend(sessions.poll());
        if sessions.finished() {
            break;
        }
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
    }
    assert_eq!(outcomes.len(), MAX_SESSIONS);
    assert!(outcomes
        .iter()
        .all(|event| event.origin == "https://localhost:9"
            && event.outcome.error_message().is_none()));
    assert_eq!(sessions.wait_handles().count(), 0);
    assert!(sessions.poll().is_empty());
}
#[test]
fn terminal_classification_keeps_revocation_and_window_failures_visible() {
    use crate::{live_relay_control_runtime::Exit, live_relay_viewer_runtime::Exit as MediaExit};
    for exit in [Exit::Stopped, Exit::Closed, Exit::Media(MediaExit::Closed)] {
        assert!(Outcome {
            exit,
            window_error: None
        }
        .error_message()
        .is_none());
    }
    assert!(Outcome {
        exit: Exit::Media(MediaExit::DeviceRevoked),
        window_error: None
    }
    .error_message()
    .unwrap()
    .contains("DeviceRevoked"));
    assert_eq!(
        Outcome {
            exit: Exit::Stopped,
            window_error: Some("window failure".into())
        }
        .error_message()
        .as_deref(),
        Some("window failure")
    );
}

#[test]
fn discovery_shares_collection_capacity_and_is_reaped_on_close() {
    let config = || {
        Config::new(
            "https://localhost:9",
            std::path::Path::new(
                "C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-relay-profile",
            ),
            "test",
        )
        .unwrap()
    };
    let mut sessions = Sessions::default();
    sessions.discover(config(), revoked()).unwrap();
    assert!(matches!(sessions.discover(config(), revoked()), Err(error)
        if error.kind() == io::ErrorKind::WouldBlock));
    for _ in 1..MAX_SESSIONS {
        start(&mut sessions).unwrap();
    }
    assert_eq!(sessions.wait_handles().count(), 7);
    assert!(matches!(start(&mut sessions), Err(error)
        if error.kind() == io::ErrorKind::WouldBlock));
    sessions.cancel_discovery();
    assert!(sessions.take_catalog().is_none());
    sessions.close();
    assert!(matches!(sessions.discover(config(), revoked()), Err(error)
        if error.kind() == io::ErrorKind::BrokenPipe));
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    while !sessions.finished() {
        assert!(sessions.poll_catalog().is_none());
        sessions.poll();
        if sessions.finished() {
            break;
        }
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
    }
    assert_eq!(sessions.wait_handles().count(), 0);
    assert!(sessions.poll_catalog().is_none());
}

#[test]
fn projection_inbox_shares_capacity_and_retains_cancelled_job_until_reaped() {
    projection_capacity(false);
}
#[test]
fn projection_targets_share_capacity_and_retain_cancelled_job_until_reaped() {
    projection_capacity(true);
}
fn projection_capacity(targets: bool) {
    let mut sessions = Sessions::default();
    if targets {
        sessions
            .query_projection_targets(surface_config(), revoked())
            .unwrap();
    } else {
        sessions
            .query_projection_inbox(surface_config(), revoked())
            .unwrap();
    }
    assert_eq!(sessions.projection_targets_pending(), targets);
    assert!(
        matches!(sessions.query_projection_targets(surface_config(), revoked()),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
    assert!(
        matches!(sessions.query_projection_inbox(surface_config(), revoked()),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
    for _ in 1..MAX_SESSIONS {
        start(&mut sessions).unwrap();
    }
    assert_eq!(sessions.wait_handles().count(), 7);
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    assert!(matches!(start(&mut sessions), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    sessions.cancel_projection();
    assert!(sessions.projection_pending());
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    sessions.close();
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    assert!(
        matches!(sessions.query_projection_targets(surface_config(), revoked()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe)
    );
    assert!(
        matches!(sessions.query_projection_inbox(surface_config(), revoked()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe)
    );
    while !sessions.finished() {
        assert!(sessions.poll_projection().is_none());
        sessions.poll();
        if sessions.finished() {
            break;
        }
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
    }
    assert!(!sessions.projection_pending());
    assert_eq!(sessions.wait_handles().count(), 0);
    assert_eq!(sessions.occupied(), 0);
}
