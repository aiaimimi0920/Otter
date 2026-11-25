//! 不可变附件、CAS、宿主显隐、损坏数据保留及 payload 摘要回归。
use super::*;
use serde_json::json;
pub fn attachment() -> Value {
    json!({"attachmentId":"neuro.official/ocr.result","typeId":"neuro.official/ocr.result.v1","schemaVersion":"1","revision":1,"pluginId":"neuro.official/ocr","pluginVersion":"1.0.0","payload":{"fullText":"示例","visible":true},"resourceRefs":[]})
}
pub fn store(items: Vec<Value>) -> Store {
    let raw = RawValue::from_string(
        serde_json::to_string(&json!({"schemaVersion":1,"revision":8,"attachments":items}))
            .unwrap(),
    )
    .unwrap();
    Store::restore(Some(&raw))
}
fn snapshot() -> Snapshot {
    let mut value = crate::loom_extension_snapshot::tests::fixture();
    value["contributions"]["dataTypes"] = json!([{"id":"neuro.official/ocr.result.v1","pluginId":"neuro.official/ocr","scopeId":"scope:ocr"}]);
    Snapshot::parse(&serde_json::to_vec(&value).unwrap()).unwrap()
}
#[test]
fn visibility_is_host_only_and_clones_share_immutable_payload_and_stamp() {
    let mut s = store(vec![attachment()]);
    let old = s.clone();
    let stamp = s.stamp();
    let payload = s.items()[0].attachment.payload().unwrap() as *const Value;
    s.visibility(&[("neuro.official/ocr.result", false)])
        .unwrap();
    assert!(!stamp.matches(&s));
    assert!(stamp.matches(&old));
    assert_eq!(
        payload,
        s.items()[0].attachment.payload().unwrap() as *const Value
    );
    assert!(old.items()[0].is_visible());
    assert!(!s.items()[0].is_visible());
    assert_eq!(s.items()[0].attachment.revision(), 1);
    assert_eq!(
        s.items()[0].attachment.payload(),
        old.items()[0].attachment.payload()
    );
    let mut fields = Map::new();
    s.write(&mut fields).unwrap();
    assert_eq!(fields["extensionState"]["revision"], 9);
    let wire = serde_json::to_value(&s.items()[0].attachment).unwrap();
    assert!(wire.get("overlayVisible").is_none() && wire.get("payloadDigest").is_none());
}
#[test]
fn invalid_restore_is_retained_but_never_activated_or_overwritten() {
    for value in [
        json!(null),
        json!({"schemaVersion":2,"future":[1,2]}),
        json!({"schemaVersion":1,"revision":0,"attachments":[attachment(),attachment()]}),
    ] {
        let raw = RawValue::from_string(value.to_string()).unwrap();
        let mut s = Store::restore(Some(&raw));
        assert!(!s.available());
        assert!(s.items().is_empty());
        assert_eq!(s.visibility(&[("anything", true)]), Err(Error::Unavailable));
        let mut fields = Map::new();
        s.write(&mut fields).unwrap();
        assert_eq!(fields["extensionState"], value);
    }
}
#[test]
fn cas_validates_owner_declarations_source_and_revision_and_remove_does_not_resurrect() {
    let catalog = snapshot();
    let mut s = Store::default();
    let stamp = s.stamp();
    let mut value = attachment();
    value["priorRevision"] = 0.into();
    assert_eq!(
        s.upsert(&catalog, "scope:ocr", 2, 3, value.clone()),
        Err(Error::Stale)
    );
    assert!(stamp.matches(&s));
    s.upsert(&catalog, "scope:ocr", 3, 3, value.clone())
        .unwrap();
    assert!(s.items()[0].digest.is_some());
    assert_eq!(
        s.upsert(&catalog, "scope:ocr", 3, 3, value.clone()),
        Err(Error::Conflict)
    );
    value["priorRevision"] = 1.into();
    value["revision"] = 2.into();
    value["typeId"] = "undeclared".into();
    assert_eq!(
        s.upsert(&catalog, "scope:ocr", 3, 3, value),
        Err(Error::Owner)
    );
    assert_eq!(
        s.remove(&catalog, "wrong", 3, 3, "neuro.official/ocr.result", 1),
        Err(Error::Owner)
    );
    assert_eq!(
        s.remove(&catalog, "scope:ocr", 3, 3, "neuro.official/ocr.result", 0),
        Err(Error::Conflict)
    );
    s.remove(&catalog, "scope:ocr", 3, 3, "neuro.official/ocr.result", 1)
        .unwrap();
    let mut fields = json!({"extensionState":{"old":true}})
        .as_object()
        .unwrap()
        .clone();
    s.write(&mut fields).unwrap();
    assert_eq!(fields["extensionState"]["attachments"], json!([]));
}
#[test]
fn digest_and_unknown_fields_roundtrip_but_tampered_payload_is_unavailable() {
    let mut a = attachment();
    a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
    a["future"] = json!({"keep":true});
    let s = store(vec![a.clone()]);
    assert!(s.available());
    let mut fields = Map::new();
    s.write(&mut fields).unwrap();
    assert_eq!(
        fields["extensionState"]["attachments"][0]["future"]["keep"],
        true
    );
    a["payload"]["fullText"] = "changed".into();
    assert!(!store(vec![a]).available());
}
