//! 隔离日志的显式停止/成功事实/重启/精确 ack；Active::retire 在此只模拟已确认 HTTP 的调用边界。
use super::*;
use crate::device_authorization_lease::Grant;
use binding::{stop::Acknowledgement, tests as fixture};
use serde_json::{json, Value};
fn grant(request: &Request) -> Grant {
    Grant::new(crate::device_session_client::tests::session(
        request.intent().envelope().origin(),
        u64::MAX,
    ))
}
fn confirm_unlink(request: &Request) {
    let mut grant = grant(request);
    let origin = request.intent().envelope().origin();
    let active = request
        .activate(grant.lease(origin, 0).unwrap().into(), true)
        .unwrap();
    active.retire().unwrap();
}
fn rejects_activate(request: &Request, cleanup: bool) {
    let mut grant = grant(request);
    let lease = grant
        .lease(request.intent().envelope().origin(), 0)
        .unwrap()
        .into();
    assert!(request.activate(lease, cleanup).is_err());
}
fn proof(source: &crate::session_source::Source, intent: &Intent) -> Acknowledgement {
    let index = binding::checkpoint::Index::capture(source).unwrap();
    Acknowledgement::committed(&index, source, intent.scope(), intent).unwrap()
}
#[test]
fn explicit_stop_receipt_survives_restart_and_exact_ack_without_network_or_resigning() {
    let scope = "a".repeat(64);
    let source = fixture::source("unit-1", &scope);
    let (directory, store, intent) = fixture::signed("unit-1", &scope, "https://loom.example");
    let envelope = intent.envelope().clone();
    let request = Request::new(intent.clone(), directory.identity());
    let ack = proof(&source, &intent);
    assert_eq!(request.acknowledge_stop(&ack), Err(Error::Changed));
    request.request_stop().unwrap();
    assert!(request.stop_requested());
    assert!(!request.published_unlinked());
    assert_eq!(request.acknowledge_stop(&ack), Err(Error::Changed));
    let bytes = std::fs::read(directory.path()).unwrap();
    let pending: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(pending["entries"][0]["stop"], "requested");
    assert_eq!(pending["entries"][0]["cancelled"], true);
    rejects_activate(&request, false);
    drop((request, intent, store));
    let reopened = Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let intent = reopened.pending(&AtomicBool::new(false)).unwrap().remove(0);
    assert_eq!(intent.envelope(), &envelope);
    let request = Request::new(intent, directory.identity());
    assert!(request.stop_requested());
    confirm_unlink(&request);
    assert!(request.published_unlinked());
    assert!(
        !request.published_retired(),
        "receipt cannot disappear before canonical ack"
    );
    rejects_activate(&request, true);
    let saved: Value = serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    assert_eq!(
        saved["entries"][0]["snapshot"],
        pending["entries"][0]["snapshot"]
    );
    assert_eq!(
        saved["entries"][0]["envelope"],
        pending["entries"][0]["envelope"]
    );
    drop((request, reopened));
    let store = Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let request = Request::new(
        store.pending(&AtomicBool::new(false)).unwrap().remove(0),
        directory.identity(),
    );
    assert!(
        request.published_unlinked(),
        "terminal receipt is durable, not missing-Request inference"
    );
    request.acknowledge_stop(&ack).unwrap();
    assert!(request.published_retired());
    assert!(store.pending(&AtomicBool::new(false)).unwrap().is_empty());
    let next = super::super::tests::prepare(&store, &directory, &super::super::tests::selection(1))
        .unwrap();
    assert_ne!(next.envelope(), &envelope);
    let before = std::fs::read(directory.path()).unwrap();
    assert_eq!(request.acknowledge_stop(&ack), Err(Error::Changed));
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
}
#[test]
fn stop_patch_preserves_unknown_fields_and_requires_exact_pending_before_removal() {
    let scope = "a".repeat(64);
    let (directory, _store, intent) = fixture::signed("unit-1", &scope, "https://loom.example");
    let original = fixture::source("unit-1", &scope);
    let candidate = fixture::candidate(&original, &intent, 1, intent.digest());
    let mut value: Value = serde_json::from_slice(candidate.source()).unwrap();
    value["stickers"][0]["projectionSenders"][0]["future"] = json!({"keep":[1,2]});
    value["stickers"][0]["futureImage"] = "x".repeat(1024 * 1024).into();
    let source: crate::session_source::Source = serde_json::to_vec(&value).unwrap().into();
    let binding = binding::checkpoint::Index::capture(&source)
        .unwrap()
        .entries()[0]
        .clone();
    assert!(binding::stop::prepare(&source, &binding, true, &AtomicBool::new(false)).is_err());
    assert!(binding::stop::prepare(&source, &binding, false, &AtomicBool::new(true)).is_err());
    let pending =
        binding::stop::prepare(&source, &binding, false, &AtomicBool::new(false)).unwrap();
    let index = binding::checkpoint::Index::capture(&pending).unwrap();
    let expected = index.for_intent(&intent).unwrap();
    assert!(expected.stopped());
    assert!(Acknowledgement::committed(&index, &pending, &scope, &intent).is_none());
    assert!(binding::stop::prepare(&pending, &binding, true, &AtomicBool::new(false)).is_err());
    let result = binding::stop::prepare(&pending, expected, true, &AtomicBool::new(false)).unwrap();
    let mut saved: Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(saved["stickers"][0]["projectionSenders"], json!([]));
    saved["stickers"][0]["projectionSenders"] = value["stickers"][0]["projectionSenders"].clone();
    assert_eq!(saved, value, "only the exact sender entry may change");
    let index = binding::checkpoint::Index::capture(&result).unwrap();
    assert!(Acknowledgement::committed(&index, &result, &"b".repeat(64), &intent).is_none());
    let copy = result.to_vec().into();
    assert!(Acknowledgement::committed(&index, &copy, &scope, &intent).is_none());
    assert!(Acknowledgement::committed(&index, &result, &scope, &intent).is_some());
    assert!(
        binding::Binding::admit_send(&result, &super::super::tests::selection(1)).is_err(),
        "preserved primary receiver still forbids sending; stop removal cannot change its role"
    );
    assert!(directory.path().exists());
}
#[test]
fn invalid_receipt_flags_and_external_disk_edits_fail_closed() {
    let (directory, store) = super::super::tests::setup();
    let intent =
        super::super::tests::prepare(&store, &directory, &super::super::tests::selection(1))
            .unwrap();
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    assert!(
        value["entries"][0].get("stop").is_none(),
        "old ordinary records keep their wire format"
    );
    for stop in [
        json!("unlinked"),
        json!("requested"),
        json!("unknown"),
        json!(true),
    ] {
        value["entries"][0]["stop"] = stop;
        assert!(Document::decode(
            &serde_json::to_vec(&value).unwrap(),
            &AtomicBool::new(false)
        )
        .is_err());
    }
    let request = Request::new(intent, directory.identity());
    request.request_stop().unwrap();
    confirm_unlink(&request);
    let ack = proof(
        &fixture::source("unit-1", &"a".repeat(64)),
        request.intent(),
    );
    let mut changed = std::fs::read(directory.path()).unwrap();
    changed.push(b' ');
    std::fs::write(directory.path(), &changed).unwrap();
    assert_eq!(request.acknowledge_stop(&ack), Err(Error::Changed));
    assert!(!request.published_retired());
    assert_eq!(std::fs::read(directory.path()).unwrap(), changed);
}
