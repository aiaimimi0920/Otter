use super::*;
use crate::surface_live_binding::Binding;
#[path = "surface_attach_http_tests.rs"]
mod attach;
#[path = "surface_attach_runtime_tests.rs"]
mod attach_runtime;
#[path = "surface_event_http_tests.rs"]
mod event;
#[path = "surface_lifecycle_http_tests.rs"]
mod lifecycle;
#[path = "surface_lifecycle_runtime_tests.rs"]
mod lifecycle_runtime;
#[path = "surface_recovery_http_tests.rs"]
mod recovery;
#[path = "surface_resource_http_tests.rs"]
mod resource;
#[path = "surface_stream_http_tests.rs"]
mod stream;
fn mounted() -> serde_json::Value {
    serde_json::json!({"instance":{"descriptor":{"instanceId":"surface:test/part","generation":2},
        "attachments":{"attachment:1":{"descriptor":{"attachmentId":"attachment:1",
            "instanceId":"surface:test/part","deviceId":"device-1","hookNodeId":"node:1"},
            "lifecycle":"mounted","lifecycleRevision":7,
            "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"surface:test/part",
                "attachmentId":"attachment:1","revision":3,"scene":{"id":"root","type":"text",
                    "props":null,"layout":null,"style":null,"accessibility":null}}}}}})
}
fn binding(origin: &str) -> Binding {
    Binding::from_mount(
        &serde_json::to_vec(&mounted()).unwrap(),
        origin,
        "device-1",
        "node:1",
    )
    .unwrap()
}
#[test]
fn authenticated_remount_uses_fixed_origin_nonce_and_escaped_instance() {
    let (listener, client, config) = fixture();
    let expected = binding(&client.origin);
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/mount "));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret"));
        assert!(head.to_ascii_lowercase().contains("x-loom-device-nonce:"));
        assert_eq!(body, serde_json::json!({"attachmentId":"attachment:1"}));
        reply(&mut stream, 200, &serde_json::to_vec(&mounted()).unwrap());
    });
    let cancel = Cancel::new();
    let result = Control::new(&client, &session, &cancel)
        .remount_surface(&expected)
        .unwrap();
    assert_eq!(result.binding, expected);
    assert!(result.snapshot.get().contains("scene"));
    // 保留 Loom 的原始 null 快照，由生产场景边界解释，而不是 HTTP 层删除字段。
    let raw: serde_json::Value = serde_json::from_str(result.snapshot.get()).unwrap();
    assert_eq!(raw["scene"]["props"], serde_json::Value::Null);
    let scene = result.declarative_scene().unwrap();
    assert_eq!(scene.node_count(), 1);
    assert!(scene.root().props().is_empty());
    peer.join().unwrap();
}
#[test]
fn remount_rejects_foreign_origin_before_io_and_snapshot_identity_substitution() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert!(matches!(
        control.remount_surface(&binding("https://foreign.example")),
        Err(LiveError::SurfaceBinding(_))
    ));
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        let mut response = mounted();
        response["instance"]["attachments"]["attachment:1"]["snapshot"]["attachmentId"] =
            "other".into();
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    });
    assert!(matches!(
        control.remount_surface(&binding(&client.origin)),
        Err(LiveError::SurfaceBinding(_))
    ));
    peer.join().unwrap();
}
