//! CAS构造/单调版本/取消预算保护；预期字段独立构造，不调用新实现作oracle。
use super::*;
use crate::projection_delivery::sender::tests::{prepare, selection, setup};
use serde_json::{json, Value};
fn response(intent: &Intent, revision: u64, digest: &str) -> Response {
    Response::parse(
        &serde_json::to_vec(&json!({
            "envelope":intent.envelope(),"revision":revision,"digest":digest,"linked":true,
            "receiverDeviceId":null,"receiverUnitId":null,"snapshot":null,
            "delivery":{"targetDeviceId":intent.target(),"status":"awaiting_confirmation"}
        }))
        .unwrap(),
        intent,
        false,
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn continuous_update_equal_digest_is_noop_and_exhausted_revision_never_wraps() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let original = response(&intent, 1, intent.digest());
    assert!(Update::prepare(
        super::super::tests::frame(),
        &original,
        &AtomicBool::new(false)
    )
    .unwrap()
    .is_none());
    let max = response(&intent, envelope::MAX_REVISION, &"b".repeat(64));
    assert_eq!(
        Update::prepare(super::super::tests::frame(), &max, &AtomicBool::new(false)).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Update::prepare(super::super::tests::frame(), &max, &AtomicBool::new(true)).err(),
        Some(Error::Stopped)
    );
}
#[test]
fn continuous_update_requires_exact_intent_and_monotonic_binding_baseline() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let original = response(&intent, 1, intent.digest());
    let mut source: Value =
        serde_json::from_slice(&binding::tests::source("unit-1", &"a".repeat(64))).unwrap();
    source["stickers"][0]
        .as_object_mut()
        .unwrap()
        .remove("qrProjection");
    let source = serde_json::to_vec(&source).unwrap().into();
    let candidate = binding::Binding::prepare(
        &source,
        &intent,
        &original,
        "target",
        &AtomicBool::new(false),
    )
    .unwrap();
    validate(candidate.binding(), &intent, &original).unwrap();
    binding::Binding::admit_update(candidate.source(), candidate.binding()).unwrap();
    assert_eq!(
        binding::Binding::admit_update(&source, candidate.binding()),
        Err(Error::Changed)
    );
    let mut receiver: Value = serde_json::from_slice(candidate.source()).unwrap();
    receiver["stickers"][0]["qrProjection"] = json!({"role":"receiver"});
    assert_eq!(
        binding::Binding::admit_update(
            &serde_json::to_vec(&receiver).unwrap().into(),
            candidate.binding()
        ),
        Err(Error::Changed)
    );
    let higher = response(&intent, 2, &"b".repeat(64));
    let next = binding::Binding::prepare(
        candidate.source(),
        &intent,
        &higher,
        "target",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        validate(next.binding(), &intent, &original),
        Err(Error::Changed)
    );
    assert_eq!(
        validate(
            next.binding(),
            &intent,
            &response(&intent, 2, &"c".repeat(64))
        ),
        Err(Error::Changed)
    );
    let other = prepare(&store, &directory, &selection(2)).unwrap();
    assert_eq!(
        validate(next.binding(), &other, &higher),
        Err(Error::Changed)
    );
}
#[test]
fn continuous_update_body_binds_original_session_and_exact_next_revision_without_resigning() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let mut grant = crate::device_authorization_lease::Grant::new(
        crate::device_session_client::tests::session(intent.envelope().origin(), u64::MAX),
    );
    let lease = Arc::new(grant.lease(intent.envelope().origin(), 0).unwrap());
    let request = Request::new(intent.clone(), directory.identity());
    let active = request.activate(lease.clone(), false).unwrap();
    let prior = response(&intent, 3, &"b".repeat(64));
    let update = Update::prepare(
        super::super::tests::frame(),
        &prior,
        &AtomicBool::new(false),
    )
    .unwrap()
    .unwrap();
    let body: Value = serde_json::from_slice(&update.body(&active).unwrap()).unwrap();
    assert_eq!(body["projectionId"], intent.envelope().projection_id());
    assert_eq!(
        body["sourceSessionId"],
        serde_json::to_value(intent.envelope()).unwrap()["source"]["sessionId"]
    );
    assert_eq!(body["priorRevision"], 3);
    assert_eq!(body["revision"], 4);
    assert_eq!(body["digest"], intent.digest());
    assert_eq!(body.as_object().unwrap().len(), 6);
    assert_eq!(body["snapshot"]["width"], 2);
    assert_eq!(body["snapshot"]["height"], 1);
    assert!(body.get("envelope").is_none());
    assert!(update.matches(&response(&intent, 4, intent.digest())));
    assert!(!update.matches(&response(&intent, 5, intent.digest())));
    assert!(!update.matches(&response(&intent, 4, &"c".repeat(64))));
    lease.revoke();
    assert_eq!(update.body(&active).err(), Some(Error::Stopped));
    assert!(!store.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
}
