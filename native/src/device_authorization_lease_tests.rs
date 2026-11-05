use super::*;
#[path = "surface_mount_admission_tests.rs"]
mod mount_admission;
#[path = "device_authorization_watch_tests.rs"]
mod watch_tests;
use crate::{
    device_identity::Identity, device_session_attempt::Attempt,
    live_relay_viewer_identity::Snapshot,
};

fn grant() -> Grant {
    let mut identity = Identity::generate().unwrap();
    identity.bind("viewer:test").unwrap();
    let session = Attempt::new(&identity, "https://localhost:1234")
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"viewer:test","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            br#"{"deviceId":"viewer:test","token":"secret","expiresAtMs":100}"#,
            0,
        )
        .unwrap();
    Grant::new(session)
}
#[test]
fn local_delivery_validation_checks_origin_expiry_and_revocation() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    assert_eq!(lease.validate("https://localhost:1234", 99), Ok(()));
    assert_eq!(
        lease.validate("https://wrong.example", 0),
        Err(Error::Credential)
    );
    assert_eq!(
        lease.validate("https://localhost:1234", 100),
        Err(Error::Credential)
    );
    drop(grant);
    assert_eq!(
        lease.validate("https://localhost:1234", 0),
        Err(Error::Revoked)
    );
}
fn media() -> (Arc<ViewerSession>, Snapshot) {
    let bytes = serde_json::to_vec(
        &serde_json::json!({"session":{"protocolVersion":"loom.live.v1",
        "sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test",
        "revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false}),
    )
    .unwrap();
    let snapshot = Snapshot::parse(&bytes, "live:test").unwrap();
    let media =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    (media, snapshot)
}
#[test]
fn cache_drop_revokes_every_independent_request_chain_and_media() {
    let mut grant = grant();
    let first = grant.lease("https://localhost:1234", 0).unwrap();
    let second = grant.lease("https://localhost:1234", 0).unwrap();
    let (media, snapshot) = media();
    first.bind_media(&media).unwrap();
    media.activate(&snapshot).unwrap();
    let permit = media.admit(1).unwrap();
    assert_eq!(first.device_id(), "viewer:test");
    assert!(!second.is_revoked());
    drop(grant);
    assert!(first.is_revoked());
    assert!(second.is_revoked());
    assert!(media.validate(&permit).is_err());
    assert!(media.activate(&snapshot).is_err());
}
#[test]
fn lease_drop_does_not_revoke_siblings_and_releases_capacity() {
    let mut grant = grant();
    let mut leases: Vec<_> = (0..MAX_LEASES)
        .map(|_| grant.lease("https://localhost:1234", 0).unwrap())
        .collect();
    assert!(matches!(
        grant.lease("https://localhost:1234", 0),
        Err(Error::Capacity)
    ));
    leases.pop();
    let replacement = grant.lease("https://localhost:1234", 0).unwrap();
    assert!(!replacement.is_revoked());
    assert!(leases.iter().all(|lease| !lease.is_revoked()));
    assert!(matches!(
        grant.lease("https://wrong.example", 0),
        Err(Error::Credential)
    ));
    assert!(matches!(
        grant.lease("https://localhost:1234", 100),
        Err(Error::Credential)
    ));
}
#[test]
fn binding_is_single_use_and_after_revocation_fails_closed() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let (first, _) = media();
    let (second, snapshot) = media();
    lease.bind_media(&first).unwrap();
    assert_eq!(lease.bind_media(&second), Err(Error::Bound));
    lease.revoke();
    assert_eq!(lease.bind_media(&second), Err(Error::Revoked));
    assert!(second.activate(&snapshot).is_err());
}
#[test]
fn binding_and_activation_racing_cache_revocation_never_leave_active_media() {
    for _ in 0..32 {
        let mut grant = grant();
        let lease = grant.lease("https://localhost:1234", 0).unwrap();
        let (media, snapshot) = media();
        let worker_media = media.clone();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = barrier.clone();
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            if lease.bind_media(&worker_media).is_ok() {
                let _ = worker_media.activate(&snapshot);
            }
            lease
        });
        barrier.wait();
        drop(grant);
        let lease = worker.join().unwrap();
        assert!(lease.is_revoked());
        assert!(media.admit(1).is_err());
    }
}
