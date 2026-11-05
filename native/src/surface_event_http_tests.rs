//! 本地真实 TLS 事件提交：固定来源认证、单次提交和未确认错误；不访问真实 Loom。
use super::*;
#[path = "surface_event_batch_http_tests.rs"]
mod batch;
#[path = "surface_convergence_http_tests.rs"]
mod convergence;
#[path = "surface_event_runtime_tests.rs"]
mod runtime;
use crate::{
    live_relay_http::SubmitError,
    surface_click_event::{ClickEvent, Modifiers},
    surface_click_route::ClickRoute,
    surface_lifecycle_owner::Owner,
    surface_scene::Scene,
};
fn active(origin: &str) -> Owner {
    let mut response = mounted();
    response["instance"]["attachments"]["attachment:1"]["lifecycle"] = "active".into();
    Owner::new(
        Binding::from_mount(
            &serde_json::to_vec(&response).unwrap(),
            origin,
            "device-1",
            "node:1",
        )
        .unwrap(),
    )
}
fn event(owner: &Owner) -> ClickEvent {
    let scene = Scene::parse(br#"{"id":"root","type":"view","events":{"click":"run"}}"#).unwrap();
    ClickRoute::new(&scene, Some("root"))
        .next()
        .unwrap()
        .encode(owner.binding(), 3, Modifiers::default())
        .unwrap()
}
#[test]
fn click_posts_exact_event_with_device_auth_and_accepts_matching_ack_once() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    let expected: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/events "));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret"));
        assert!(head.to_ascii_lowercase().contains("x-loom-device-nonce:"));
        assert_eq!(body, expected);
        reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
            "eventId":body["eventId"],"requestId":"request:1","accepted":true,"status":"accepted"
        })).unwrap());
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let ack = control.submit_surface_click(&owner, 3, &mut event).unwrap();
    assert!(event.attempted());
    assert!(ack.accepted());
    assert!(!ack.succeeded());
    assert!(matches!(
        control.submit_surface_click(&owner, 3, &mut event),
        Err(SubmitError::AlreadyAttempted)
    ));
    peer.join().unwrap();
}
#[test]
fn stale_inactive_and_foreign_events_are_rejected_before_attempt() {
    let (_listener, client, _config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let owner = active(&client.origin);
    let mut pending = event(&owner);
    assert!(matches!(
        control.submit_surface_click(&owner, 4, &mut pending),
        Err(SubmitError::Stale)
    ));
    let inactive = Owner::new(binding(&client.origin));
    assert!(matches!(
        control.submit_surface_click(&inactive, 3, &mut pending),
        Err(SubmitError::Inactive)
    ));
    assert!(!pending.attempted());
    let foreign = active("https://foreign.example");
    assert!(matches!(
        control.submit_surface_click(&foreign, 3, &mut pending),
        Err(SubmitError::Stale)
    ));
    let mut pending = event(&foreign);
    assert!(matches!(
        control.submit_surface_click(&foreign, 3, &mut pending),
        Err(SubmitError::Identity)
    ));
    assert!(!pending.attempted());
}
#[test]
fn mismatched_ack_is_unconfirmed_and_cannot_trigger_replay() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, br#"{"protocolVersion":"loom.surface.v1","instanceId":"wrong","eventId":"wrong","requestId":"request:1","accepted":true,"status":"succeeded"}"#);
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert!(matches!(
        control.submit_surface_click(&owner, 3, &mut event),
        Err(SubmitError::Ack(_))
    ));
    assert!(event.attempted());
    assert!(matches!(
        control.submit_surface_click(&owner, 3, &mut event),
        Err(SubmitError::AlreadyAttempted)
    ));
    peer.join().unwrap();
}
