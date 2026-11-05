use super::*;
use crate::{
    live_relay_http::Persistence,
    surface_attach_runtime::{Error, Outcome, Request},
    surface_host_capabilities::{Input, Report, Runtime},
};

fn request() -> Arc<Request> {
    Arc::new(
        Request::new(
            "art:test".into(),
            "node:test".into(),
            Report::new(
                vec![Runtime::Declarative],
                vec!["text".into()],
                vec![],
                vec![],
                Input::default(),
            )
            .unwrap(),
            Persistence::Persistent,
        )
        .unwrap(),
    )
}
#[test]
fn failed_delivery_restores_original_outcome_and_capacity() {
    let mut sessions = Sessions::default();
    sessions.attach_outcomes.push(Outcome {
        origin: "https://localhost:1234".into(),
        device: "device:test".into(),
        request: request(),
        binding: None,
        result: Err(Error::Stopped),
        uncertain: true,
    });
    assert!(!sessions.deliver_attach(|outcome| Err(Box::new(outcome))));
    assert_eq!(sessions.occupied(), 1);
    assert!(sessions.deliver_attach(|outcome| {
        assert!(outcome.uncertain);
        assert_eq!(outcome.request.art(), "art:test");
        Ok(())
    }));
    assert_eq!(sessions.occupied(), 0);
    assert!(!sessions.deliver_attach(|_| panic!("empty delivery")));
}
#[test]
fn recovery_queries_share_capacity_and_shutdown_retains_context() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        sessions
            .query_surface_recovery(surface_config(), revoked(), request())
            .unwrap();
    }
    assert!(
        matches!(sessions.start_attach(surface_config(), revoked(), request()), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
    sessions.close();
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
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    for _ in 0..MAX_SESSIONS {
        let outcome = sessions.take_recovery_outcome().unwrap();
        assert_eq!(
            outcome.result,
            Err(crate::surface_recovery_runtime::Error::Stopped)
        );
        assert_eq!(outcome.request.art(), "art:test");
    }
    assert_eq!(sessions.occupied(), 0);
    assert!(
        matches!(sessions.query_surface_recovery(surface_config(), revoked(), request()), Err(e) if e.kind() == io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn attach_results_hold_shared_capacity_and_close_reaps_all_handles() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        sessions
            .start_attach(surface_config(), revoked(), request())
            .unwrap();
    }
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    assert!(
        matches!(sessions.start_surface(surface_config(), revoked(), surface_owner(), crate::surface_lifecycle_runtime::Action::Recover), Err(e) if e.error.kind() == io::ErrorKind::WouldBlock)
    );
    sessions.close();
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
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    for _ in 0..MAX_SESSIONS {
        let outcome = sessions.take_attach_outcome().unwrap();
        assert_eq!(outcome.request.art(), "art:test");
        assert_eq!(outcome.device, "viewer:test");
        assert!(matches!(outcome.result, Err(Error::Stopped)));
        assert!(!outcome.uncertain);
    }
    assert_eq!(sessions.occupied(), 0);
    assert!(
        matches!(sessions.start_attach(surface_config(), revoked(), request()), Err(e) if e.kind() == io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn attach_cancel_keeps_confirmed_binding_and_unknown_creation_context() {
    let mut sessions = Sessions::default();
    for uncertain in [false, true] {
        sessions.attach_outcomes.push(Outcome {
            origin: "https://localhost:9".into(),
            device: "viewer:test".into(),
            request: request(),
            binding: (!uncertain).then(|| surface_owner().binding().clone()),
            result: Ok(serde_json::value::RawValue::from_string("{}".into()).unwrap()),
            uncertain,
        });
    }
    sessions.cancel_surface_requests();
    for uncertain in [true, false] {
        let outcome = sessions.take_attach_outcome().unwrap();
        assert_eq!(outcome.uncertain, uncertain);
        assert_eq!(outcome.binding.is_some(), !uncertain);
        assert_eq!(outcome.request.node(), "node:test");
        assert!(matches!(outcome.result, Err(Error::Stopped)));
    }
}
