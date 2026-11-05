//! Source预算失败和预撤权：真实原HTTPS创建/关闭，不启动第二媒体或捕获线程。
use super::*;
use crate::{
    device_authorization_config::Config,
    device_authorization_lease::Grant,
    live_frames::Source,
    live_relay_http::source::Publication,
    live_relay_source_runtime::{Cleanup, Exit, Runtime},
    live_texture::Texture,
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
#[path = "live_relay_source_reconnect_http_tests.rs"]
mod reconnect;
fn config(origin: &str, root: &std::path::Path) -> Config {
    Config::from_lookup(origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(root.into()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(origin.into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(root.join("fixture-ca.pem").into_os_string()),
        _ => None,
    })
    .unwrap()
}
#[test]
fn exhausted_shared_budget_closes_created_session_and_releases_consumer_and_task() {
    let (files, _, _) = pairing::Files::new();
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let origin = client.origin.clone();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let peer = std::thread::spawn(move || {
        let mut snapshot = serde_json::Value::Null;
        for stage in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if stage == 0 {
                assert!(head.starts_with("POST /v1/live/sessions "));
                snapshot = serde_json::json!({"session":body["envelope"]["payload"]["session"],"epoch":1,"closed":false});
            } else {
                assert!(head.contains("/close HTTP/1.1"));
                snapshot["closed"] = serde_json::json!(true);
            }
            reply(&mut stream, 200, &serde_json::to_vec(&snapshot).unwrap());
        }
    });
    let frames = Source::<Texture>::new();
    let budget = crate::byte_budget::Budget::new(0);
    let mut runtime = Runtime::start(
        config(&origin, files.root()),
        grant.lease(&origin, 1).unwrap(),
        Publication::new("hook:test", [1, 1], None).unwrap(),
        frames.subscribe().unwrap(),
        budget.clone(),
    )
    .unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handles().next().unwrap().as_raw_handle()),
                5000,
            )
        },
        WAIT_OBJECT_0
    );
    let result = runtime.try_join().unwrap();
    assert_eq!(result.exit, Exit::Budget);
    assert_eq!(result.cleanup, Cleanup::Confirmed);
    assert!(!runtime.ready());
    assert_eq!(budget.used(), 0);
    assert!(!frames.has_consumers());
    peer.join().unwrap();
}
#[test]
fn already_revoked_lease_never_creates_a_session_or_publishes_ready() {
    let (files, _, _) = pairing::Files::new();
    let (_listener, client, _) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 1).unwrap();
    lease.revoke();
    let frames = Source::<Texture>::new();
    let mut runtime = Runtime::start(
        config(&client.origin, files.root()),
        lease,
        Publication::new("hook:test", [1, 1], None).unwrap(),
        frames.subscribe().unwrap(),
        crate::byte_budget::Budget::new(1024),
    )
    .unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handles().next().unwrap().as_raw_handle()),
                5000,
            )
        },
        WAIT_OBJECT_0
    );
    let result = runtime.try_join().unwrap();
    assert_eq!(result.exit, Exit::Stopped);
    assert_eq!(result.cleanup, Cleanup::NotCreated);
    assert!(!runtime.ready());
    assert!(!frames.has_consumers());
}
#[test]
fn interactive_source_rejects_mismatched_grant_and_stops_bridge_on_early_exit() {
    use crate::live_relay_source_runtime::{input::Bridge, Interaction};
    let (files, _, _) = pairing::Files::new();
    let (_listener, client, _) = fixture();
    let origin = &client.origin;
    let mut grant = Grant::new(session(origin, u64::MAX));
    let mut other = Grant::new(session(origin, u64::MAX));
    let frames = Source::<Texture>::new();
    let publication = || {
        let mut p = Publication::new("hook:test", [1, 1], Some(("window:test".into(), 1))).unwrap();
        p.allow_interaction().unwrap();
        p
    };
    let bridge = Bridge::new();
    let wrong = Runtime::start_with_interaction(
        config(origin, files.root()),
        grant.lease(origin, 1).unwrap(),
        publication(),
        frames.subscribe().unwrap(),
        crate::byte_budget::Budget::new(1024),
        Some(Interaction::new(other.lease(origin, 1).unwrap(), bridge)),
    );
    assert!(matches!(wrong,Err(e) if e.kind()==std::io::ErrorKind::PermissionDenied));
    let bridge = Bridge::new();
    let lease = grant.lease(origin, 1).unwrap();
    lease.revoke();
    let mut runtime = Runtime::start_with_interaction(
        config(origin, files.root()),
        lease,
        publication(),
        frames.subscribe().unwrap(),
        crate::byte_budget::Budget::new(1024),
        Some(Interaction::new(
            grant.lease(origin, 1).unwrap(),
            bridge.clone(),
        )),
    )
    .unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handles().next().unwrap().as_raw_handle()),
                5000,
            )
        },
        WAIT_OBJECT_0
    );
    assert_eq!(runtime.try_join().unwrap().exit, Exit::Stopped);
    assert!(bridge.stopping());
    assert!(!frames.has_consumers());
}
