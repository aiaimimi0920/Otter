//! 合成身份/会话验证旧字段保留、容量、幂等身份及跨重试单调性；候选不授发布权。
use super::super::tests::{frame, Directory};
use super::*;
use serde_json::{json, Value};
pub(crate) fn source(unit: &str, scope: &str) -> Source {
    serde_json::to_vec(&json!({"stickers":[{"id":unit,"x":0,"y":0,"w":4,"h":4,
        "future":{"keep":true},"qrProjection":{"role":"receiver","unknown":[1,2,3]}}],
        "links":[],"futureRoot":{"keep":true},"documentRevision":7,
        "otterNativeProjectionScope":scope}))
    .unwrap()
    .into()
}
pub(crate) fn signed(unit: &str, scope: &str, origin: &str) -> (Directory, Store, Intent) {
    let directory = Directory::new();
    directory.install_identity();
    let store = Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let selection = Selection::new(scope, origin, unit, "target-1").unwrap();
    let intent = store
        .prepare(
            Preparation {
                selection: &selection,
                frame: frame(),
                identity_root: &directory.identity(),
                device: "device-1",
                now_ms: 1_000,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    (directory, store, intent)
}
pub(crate) fn response(intent: &Intent, revision: u64, digest: &str, linked: bool) -> Response {
    let wire = json!({"envelope":intent.envelope(),"revision":revision,"digest":digest,"linked":true,
        "receiverDeviceId":linked.then_some(intent.target()),"receiverUnitId":linked.then_some("receiver-1"),
        "delivery":{"targetDeviceId":intent.target(),"status":if linked {"displayed"} else {"awaiting_confirmation"}}});
    Response::parse(
        &serde_json::to_vec(&wire).unwrap(),
        intent,
        false,
        &AtomicBool::new(false),
    )
    .unwrap()
}
pub(crate) fn candidate(
    source: &Source,
    intent: &Intent,
    revision: u64,
    digest: &str,
) -> Candidate {
    Binding::prepare(
        source,
        intent,
        &response(intent, revision, digest, false),
        "Target",
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn value(source: &Source) -> Value {
    serde_json::from_slice(source).unwrap()
}
fn from_value(value: &Value) -> Source {
    serde_json::to_vec(value).unwrap().into()
}
fn rejects(source: &Source, intent: &Intent, revision: u64, digest: &str) {
    assert!(Binding::prepare(
        source,
        intent,
        &response(intent, revision, digest, false),
        "Target",
        &AtomicBool::new(false)
    )
    .is_err());
}
#[test]
fn candidate_preserves_unknown_fields_primary_receiver_and_original_disk_bytes() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (directory, store, intent) = signed("unit-1", &scope, "https://loom.example");
    let before = std::fs::read(directory.path()).unwrap();
    let result = candidate(&original, &intent, 1, intent.digest());
    assert!(result.expected().same_storage(&original));
    assert!(result.index().is_for(result.source()));
    assert!(result.index().includes(result.binding()));
    assert!(result.binding().matches_intent(&intent));
    assert!(!result.binding().linked(), "awaiting is not a receiver");
    let saved = value(result.source());
    assert_eq!(
        saved["documentRevision"], 7,
        "metadata patch is not a disk save"
    );
    assert_eq!(saved["futureRoot"], value(&original)["futureRoot"]);
    assert_eq!(
        saved["stickers"][0]["future"],
        value(&original)["stickers"][0]["future"]
    );
    assert_eq!(
        saved["stickers"][0]["qrProjection"],
        value(&original)["stickers"][0]["qrProjection"]
    );
    assert!(value(&original)["stickers"][0]
        .get("projectionSenders")
        .is_none());
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
    assert!(!format!("{:?}", result.binding()).contains(intent.envelope().projection_id()));
    store.cancel(&intent).unwrap();
    assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
}
#[test]
fn retries_preserve_extensions_and_reject_regression_conflict_or_stopped_binding() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (_directory, _store, intent) = signed("unit-1", &scope, "https://loom.example");
    let first = candidate(&original, &intent, 5, &"b".repeat(64));
    let mut saved = value(first.source());
    saved["stickers"][0]["projectionSenders"][0]["link"]["futureLink"] = json!([1, "keep"]);
    saved["stickers"][0]["projectionSenders"][0]["target"]["futureTarget"] = json!({"keep":true});
    let source = from_value(&saved);
    rejects(&source, &intent, 4, &"b".repeat(64));
    rejects(&source, &intent, 5, &"c".repeat(64));
    let update = candidate(&source, &intent, 6, &"c".repeat(64));
    let updated = value(update.source());
    assert_eq!(
        updated["stickers"][0]["projectionSenders"][0]["link"]["futureLink"],
        json!([1, "keep"])
    );
    assert_eq!(
        updated["stickers"][0]["projectionSenders"][0]["target"]["futureTarget"],
        json!({"keep":true})
    );
    assert_eq!(
        updated["stickers"][0]["projectionSenders"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        candidate(update.source(), &intent, 6, &"c".repeat(64)).binding(),
        update.binding()
    );
    for flag in ["stopped", "stopPending"] {
        let mut stopped = saved.clone();
        stopped["stickers"][0]["projectionSenders"][0]["link"][flag] = true.into();
        rejects(&from_value(&stopped), &intent, 6, &"c".repeat(64));
    }
}
fn legacy(template: &Value, number: usize) -> Value {
    let mut value = template.clone();
    value["link"]
        .as_object_mut()
        .unwrap()
        .remove("otterNativeSend");
    value["link"]["envelope"]["projectionId"] = format!("projection:{number:032x}").into();
    value["target"]["deviceId"] = format!("old-target-{number}").into();
    value["future"] = "preserve".into();
    value
}
#[test]
fn legacy_capacity_and_primary_source_count_without_rewriting_legacy_entries() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (_directory, _store, intent) = signed("unit-1", &scope, "https://loom.example");
    let first = candidate(&original, &intent, 1, intent.digest());
    let template = value(first.source())["stickers"][0]["projectionSenders"][0].clone();
    let mut saved = value(&original);
    let old: Vec<_> = (1..8).map(|i| legacy(&template, i)).collect();
    saved["stickers"][0]["projectionSenders"] = old.clone().into();
    let appended = candidate(&from_value(&saved), &intent, 1, intent.digest());
    assert_eq!(
        &value(appended.source())["stickers"][0]["projectionSenders"]
            .as_array()
            .unwrap()[..7],
        &old
    );
    saved["stickers"][0]["projectionSenders"] = (1..=8)
        .map(|i| legacy(&template, i))
        .collect::<Vec<_>>()
        .into();
    rejects(&from_value(&saved), &intent, 1, intent.digest());
    saved["stickers"][0]["projectionSenders"] = old.into();
    saved["stickers"][0]["qrProjection"] = legacy(&template, 99)["link"].clone();
    rejects(&from_value(&saved), &intent, 1, intent.digest());
    let escaped: Source = String::from_utf8(from_value(&saved).to_vec())
        .unwrap()
        .replace("\"role\":\"source\"", "\"role\":\"\\u0073ource\"")
        .into_bytes()
        .into();
    rejects(&escaped, &intent, 1, intent.digest());
    saved["stickers"][0]["qrProjection"] = legacy(&template, 1)["link"].clone();
    assert_eq!(
        candidate(&from_value(&saved), &intent, 1, intent.digest()).binding(),
        first.binding()
    );
}
#[test]
fn malformed_or_oversized_metadata_scope_and_duplicate_destinations_fail_closed() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (_directory, _store, intent) = signed("unit-1", &scope, "https://loom.example");
    let first = candidate(&original, &intent, 1, intent.digest());
    for malformed in [
        Value::Null,
        json!({}),
        json!([false]),
        json!([{"link":"x"}]),
    ] {
        let mut bad = value(&original);
        bad["stickers"][0]["projectionSenders"] = malformed;
        rejects(&from_value(&bad), &intent, 1, intent.digest());
    }
    let mut bad = value(&original);
    bad["otterNativeProjectionScope"] = "b".repeat(64).into();
    rejects(&from_value(&bad), &intent, 1, intent.digest());
    let mut bad = value(first.source());
    let binding = bad["stickers"][0]["projectionSenders"][0].clone();
    bad["stickers"][0]["projectionSenders"] = json!([binding, binding]);
    rejects(&from_value(&bad), &intent, 1, intent.digest());
    assert!(checkpoint::Index::capture(&from_value(&bad)).is_none());
    let mut bad = value(first.source());
    bad["stickers"][0]["projectionSenders"][0]["future"] = "x".repeat(MAX_BINDING_BYTES).into();
    rejects(&from_value(&bad), &intent, 1, intent.digest());
    assert!(checkpoint::Index::capture(&from_value(&bad)).is_none());
    assert_eq!(
        Binding::prepare(
            &original,
            &intent,
            &response(&intent, 1, intent.digest(), false),
            "Target",
            &AtomicBool::new(true)
        )
        .err(),
        Some(Error::Stopped)
    );
}
#[test]
fn index_requires_exact_allocation_marker_and_preserves_ordinary_legacy_save() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (_directory, _store, intent) = signed("unit-1", &scope, "https://loom.example");
    let result = candidate(&original, &intent, 1, intent.digest());
    let identical: Source = result.source().to_vec().into();
    assert!(!result.index().is_for(&identical));
    assert!(result.index().for_intent(&intent).is_some());
    for mutation in ["marker", "offline", "peer"] {
        let mut bad = value(result.source());
        let binding = &mut bad["stickers"][0]["projectionSenders"][0];
        match mutation {
            "marker" => binding["link"]["otterNativeSend"]["version"] = 2.into(),
            "offline" => {
                binding["link"]["offlineTransport"] = json!({"origin":"https://offline.example"})
            }
            _ => binding["target"]["peerId"] = format!("loom-{}", "a".repeat(64)).into(),
        }
        let bytes = from_value(&bad);
        assert!(checkpoint::Index::capture(&bytes).is_none());
        let decoded = crate::session_document::decode(&bytes).unwrap();
        let saved = crate::session_write::assemble(
            &bytes,
            &crate::workspace_session::Topology::capture(&decoded.workspace),
            &AtomicBool::new(false),
            |_, fields| Ok(fields),
        )
        .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&saved.bytes).unwrap()["stickers"][0]
                ["projectionSenders"],
            bad["stickers"][0]["projectionSenders"]
        );
    }
    let mut old = value(result.source());
    old["stickers"][0]["projectionSenders"][0]["link"]
        .as_object_mut()
        .unwrap()
        .remove("otterNativeSend");
    assert!(checkpoint::Index::capture(&from_value(&old))
        .unwrap()
        .entries()
        .is_empty());
}
#[test]
fn index_restore_also_enforces_primary_capacity_and_full_signed_identity() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (_directory, _store, intent) = signed("unit-1", &scope, "https://loom.example");
    let first = candidate(&original, &intent, 1, intent.digest());
    let mut saved = value(first.source());
    let template = saved["stickers"][0]["projectionSenders"][0].clone();
    let mut all: Vec<Value> = (1..8).map(|number| legacy(&template, number)).collect();
    all.push(template.clone());
    saved["stickers"][0]["projectionSenders"] = all.into();
    saved["stickers"][0]["qrProjection"] = legacy(&template, 99)["link"].clone();
    assert!(checkpoint::Index::capture(&from_value(&saved)).is_none());
    saved["stickers"][0]["qrProjection"] = legacy(&template, 1)["link"].clone();
    assert!(checkpoint::Index::capture(&from_value(&saved))
        .unwrap()
        .includes(first.binding()));
    saved["stickers"][0]["qrProjection"]["envelope"]["nonce"] = "b".repeat(32).into();
    assert!(checkpoint::Index::capture(&from_value(&saved)).is_none());
}
#[test]
fn json_escaped_names_and_legacy_identifiers_remain_compatible_without_losing_extensions() {
    let scope = "a".repeat(64);
    let original = source("unit-1", &scope);
    let (_directory, _store, intent) = signed("unit-1", &scope, "https://loom.example");
    let named = Binding::prepare(
        &original,
        &intent,
        &response(&intent, 1, intent.digest(), false),
        "Target \"中文\" \\ A",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(named.index().includes(named.binding()));
    let mut saved = value(&original);
    let template = value(named.source())["stickers"][0]["projectionSenders"][0].clone();
    saved["stickers"][0]["projectionSenders"] = json!([legacy(&template, 1)]);
    saved["stickers"][0]["futureImage"] = "large-unknown-image".repeat(64 * 1024).into();
    let escaped = String::from_utf8(from_value(&saved).to_vec())
        .unwrap()
        .replace("\"unit-1\"", "\"unit-\\u0031\"")
        .replace("\"source\"", "\"\\u0073ource\"");
    let result = candidate(&escaped.into_bytes().into(), &intent, 1, intent.digest());
    assert_eq!(
        value(result.source())["stickers"][0]["futureImage"],
        saved["stickers"][0]["futureImage"]
    );
    assert_eq!(
        value(result.source())["stickers"][0]["projectionSenders"][0],
        saved["stickers"][0]["projectionSenders"][0]
    );
}
