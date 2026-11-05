//! 不发送网络；检查资源快照的大小、身份、代际及生命周期推进。
use super::*;
pub(crate) fn id() -> String {
    format!("sha256:{}", "a".repeat(64))
}
pub(crate) fn raw(binding: &Binding, revision: u64, lease: &str) -> Box<RawValue> {
    serde_json::value::to_raw_value(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":binding.instance(),"attachmentId":binding.attachment(),
        "revision":revision,"scene":{"id":"root","type":"text","props":{"text":"resource owner"}},
        "resourceLeases":[{"leaseId":lease,"resource":{"resourceId":id(),"kind":"image",
        "mime":"application/x-neuro-rgba8","size":4,"width":1,"height":1},
        "transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{}","a".repeat(64))},"expiresAtMs":1000}]
    })).unwrap()
}
fn binding(state: &str, revision: u64, generation: u64, device: &str) -> Binding {
    let bytes=serde_json::to_vec(&serde_json::json!({"instance":{"descriptor":{"instanceId":"instance:test","generation":generation},
        "attachments":{"attachment:test":{"descriptor":{"instanceId":"instance:test","attachmentId":"attachment:test",
        "hookNodeId":"node:test","deviceId":device},"lifecycle":state,"lifecycleRevision":revision}}}})).unwrap();
    Binding::from_mount(&bytes, "https://localhost:1234", device, "node:test").unwrap()
}
#[test]
fn resource_snapshot_rebases_only_current_lifecycle_not_identity_or_revision() {
    let mounted = binding("mounted", 1, 1, "device:test");
    let active = binding("active", 2, 1, "device:test");
    let snapshot = ResourceSnapshot::new(&raw(&mounted, 7, "lease:private"), &mounted, 7).unwrap();
    snapshot.validate(&active, 7).unwrap();
    #[cfg(windows)]
    {
        let current = snapshot.mount(&active, 7).unwrap();
        assert_eq!(current.binding, active);
        assert_eq!(
            current.image_resource(&id(), 0).unwrap().id(),
            "lease:private"
        );
    }
    assert!(!format!("{snapshot:?}").contains("lease:private"));
    assert!(snapshot.validate(&active, 8).is_err());
    assert!(snapshot
        .validate(&binding("active", 2, 2, "device:test"), 7)
        .is_err());
    assert!(snapshot
        .validate(&binding("active", 2, 1, "other"), 7)
        .is_err());
    let newer = ResourceSnapshot::new(&raw(&active, 7, "lease:new"), &active, 7).unwrap();
    assert!(newer.validate(&mounted, 7).is_err());
}
#[test]
fn resource_snapshot_checks_header_and_envelope_limit() {
    let binding = binding("mounted", 1, 1, "device:test");
    assert!(ResourceSnapshot::new(&raw(&binding, 7, "lease:test"), &binding, 8).is_err());
    assert!(ResourceSnapshot::new(&raw(&binding, 0, "lease:test"), &binding, 0).is_err());
    let malformed = RawValue::from_string("{}".into()).unwrap();
    assert!(ResourceSnapshot::new(&malformed, &binding, 7).is_err());
    let mut value: serde_json::Value =
        serde_json::from_str(raw(&binding, 7, "lease:test").get()).unwrap();
    value["attachmentId"] = "other".into();
    assert!(ResourceSnapshot::new(
        &serde_json::value::to_raw_value(&value).unwrap(),
        &binding,
        7
    )
    .is_err());
    value["extra"] = "x".repeat(crate::surface_scene::MAX_BYTES).into();
    assert!(matches!(
        ResourceSnapshot::new(
            &serde_json::value::to_raw_value(&value).unwrap(),
            &binding,
            7
        ),
        Err(Error::Limit)
    ));
}
