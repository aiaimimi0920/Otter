use super::*;
#[test]
fn local_art_view_uses_explicit_host_identity_not_a_fabricated_remote_response() {
    let mut request = crate::art_local_requests::tests::request().binding;
    let local = Binding::local_art(&request, "node:1", "view:1").unwrap();
    assert_eq!(local.origin(), request.origin);
    assert_eq!(local.device(), request.device_id);
    assert_eq!(local.instance(), request.instance_id);
    assert_eq!(local.node(), "node:1");
    assert_eq!(local.attachment(), "view:1");
    assert_eq!(local.lifecycle(), Lifecycle::Active);
    assert!(Binding::local_art(&request, "", "view:1").is_err());
    request.generation = 0;
    assert!(Binding::local_art(&request, "node:1", "view:1").is_err());
}
use serde_json::{json, Value};
fn attachment(id: &str, device: &str, node: &str) -> Value {
    json!({"descriptor":{"attachmentId":id,"instanceId":"instance:1","deviceId":device,"hookNodeId":node},
        "lifecycle":"mounted","lifecycleRevision":7,"snapshot":{"revision":11}})
}
fn mounted() -> Value {
    json!({"instance":{"descriptor":{"instanceId":"instance:1","generation":2},"attachments":{
        "attachment:old":attachment("attachment:old","device:old","node:1"),
        "attachment:current":attachment("attachment:current","device:new","node:1")}}})
}
fn parse(value: &Value) -> Result<Binding, Error> {
    Binding::from_mount(
        &serde_json::to_vec(value).unwrap(),
        "https://loom.example",
        "device:new",
        "node:1",
    )
}
#[test]
fn selects_current_device_not_retained_old_identity_and_binds_context() {
    let binding = parse(&mounted()).unwrap();
    assert_eq!(binding.attachment(), "attachment:current");
    assert_eq!(binding.instance(), "instance:1");
    assert_eq!(binding.next_lifecycle_revision(), Ok(8));
    assert!(binding.matches("https://loom.example", "device:new", "node:1", 2));
    let snapshot = crate::live_relay_viewer_identity::Snapshot::parse(
        br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:1","sourceDeviceId":"source:1","sourceHookId":"hook:1","revision":1,"viewerDevices":[]},"epoch":1,"closed":false}"#,
        "live:1",
    ).unwrap();
    let target = binding
        .viewer_target(
            snapshot.identity(),
            "https://loom.example",
            "device:new",
            "node:1",
            2,
        )
        .unwrap();
    assert_eq!(target.surface, "instance:1");
    assert_eq!(target.attachment, "attachment:current");
    assert_eq!(target.selected.as_ref(), Some(snapshot.identity()));
    assert!(target.expected.is_none());
    assert!(binding
        .viewer_target(
            snapshot.identity(),
            "https://loom.example",
            "device:old",
            "node:1",
            2
        )
        .is_err());
    for (origin, device, node, generation) in [
        ("https://other.example", "device:new", "node:1", 2),
        ("https://loom.example", "device:old", "node:1", 2),
        ("https://loom.example", "device:new", "node:2", 2),
        ("https://loom.example", "device:new", "node:1", 3),
    ] {
        assert!(!binding.matches(origin, device, node, generation));
    }
}
#[test]
fn rejects_wrong_ownership_disposed_ambiguous_or_conflicting_descriptor() {
    for (field, value) in [
        ("deviceId", json!("other")),
        ("hookNodeId", json!("other")),
        ("instanceId", json!("other")),
        ("attachmentId", json!("other")),
    ] {
        let mut input = mounted();
        input["instance"]["attachments"]["attachment:current"]["descriptor"][field] = value;
        assert!(parse(&input).is_err());
    }
    let mut input = mounted();
    input["instance"]["attachments"]["attachment:current"]["lifecycle"] = json!("disposed");
    assert_eq!(parse(&input), Err(Error::Disposed));
    input = mounted();
    input["instance"]["attachments"]["attachment:second"] =
        attachment("attachment:second", "device:new", "node:1");
    assert_eq!(parse(&input), Err(Error::Ambiguous));
}
#[test]
fn response_and_revision_are_bounded_and_invalid_origins_are_rejected() {
    assert_eq!(
        Binding::from_mount(
            &vec![b' '; MAX_RESPONSE + 1],
            "https://loom.example",
            "device:new",
            "node:1"
        ),
        Err(Error::Limit)
    );
    let mut input = mounted();
    input["instance"]["attachments"]["attachment:current"]["lifecycleRevision"] = json!(u64::MAX);
    assert_eq!(
        parse(&input).unwrap().next_lifecycle_revision(),
        Err(Error::Revision)
    );
    assert_eq!(
        Binding::from_mount(b"{}", "http://loom.example", "device:new", "node:1"),
        Err(Error::Input)
    );
}
