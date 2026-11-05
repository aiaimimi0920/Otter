//! scope 格式、完整会话兼容及字节凭证；不访问磁盘、身份或网络。
use super::*;
use crate::{session_write, workspace_session::Topology};
use std::sync::atomic::AtomicBool;

fn scope(byte: char) -> Scope {
    serde_json::from_value(serde_json::Value::String(byte.to_string().repeat(64))).unwrap()
}
fn document(value: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "stickers":[], "links":[], (FIELD):value
    }))
    .unwrap()
}
#[test]
fn missing_is_legacy_but_explicit_invalid_values_never_become_fresh_identity() {
    let legacy = br#"{"stickers":[],"links":[]}"#;
    assert!(crate::session_document::decode(legacy)
        .unwrap()
        .projection_scope
        .is_none());
    for value in [
        serde_json::Value::Null,
        serde_json::json!(12),
        serde_json::json!({"scope":"a".repeat(64)}),
        serde_json::json!(["a".repeat(64)]),
        serde_json::json!("0".repeat(64)),
        serde_json::json!("A".repeat(64)),
        serde_json::json!("g".repeat(64)),
        serde_json::json!("a".repeat(63)),
        serde_json::json!("a".repeat(65)),
        serde_json::json!("中".repeat(64)),
        serde_json::json!("a".repeat(64) + "\n"),
    ] {
        assert!(crate::session_document::decode(&document(value)).is_err());
    }
    let duplicate = format!(
        r#"{{"stickers":[],"links":[],"{FIELD}":"{}","{FIELD}":"{}"}}"#,
        "a".repeat(64),
        "b".repeat(64)
    );
    assert!(crate::session_document::decode(duplicate.as_bytes()).is_err());
    let oversized = format!(
        r#"{{"stickers":[],"links":[],"{FIELD}":"{}"}}"#,
        "\\u0061".repeat(4096)
    );
    assert!(crate::session_document::decode(oversized.as_bytes()).is_err());
    let escaped = format!(
        r#"{{"stickers":[],"links":[],"{FIELD}":"{}"}}"#,
        "\\u0061".repeat(64)
    );
    assert_eq!(
        crate::session_document::decode(escaped.as_bytes())
            .unwrap()
            .projection_scope,
        Some(scope('a'))
    );
}
#[test]
fn exact_scope_is_preserved_through_native_and_compatibility_writers() {
    let cancel = AtomicBool::new(false);
    let original = br#"{"documentRevision":7,"stickers":[],"links":[],"future":{"keep":true}}"#;
    let topology = Topology::capture(&crate::session_document::decode(original).unwrap().workspace);
    let token = scope('a');
    let candidate = session_write::assemble_owned_at_revision(
        original,
        &topology,
        &cancel,
        None,
        None,
        Some(token),
        |_, _| unreachable!(),
    )
    .unwrap();
    assert_eq!(candidate.expected_revision, 7);
    assert_eq!(
        crate::session_document::decode(&candidate.bytes)
            .unwrap()
            .projection_scope,
        Some(token)
    );
    let raw: serde_json::Value = serde_json::from_slice(&candidate.bytes).unwrap();
    assert_eq!(raw["future"]["keep"], true);
    let second =
        session_write::assemble(&candidate.bytes, &topology, &cancel, |_, _| unreachable!())
            .unwrap();
    assert_eq!(
        crate::session_document::decode(&second.bytes)
            .unwrap()
            .projection_scope,
        Some(token)
    );
    assert!(matches!(
        session_write::assemble_owned_at_revision(
            &second.bytes,
            &topology,
            &cancel,
            None,
            None,
            Some(scope('b')),
            |_, _| unreachable!()
        ),
        Err(session_write::Error::Source)
    ));
    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        session_write::assemble_owned_at_revision(
            original,
            &topology,
            &cancelled,
            None,
            None,
            Some(token),
            |_, _| unreachable!()
        ),
        Err(session_write::Error::Cancelled)
    ));
}
#[test]
fn proof_is_exact_allocation_not_same_text_or_a_path() {
    let source: Source = document(serde_json::json!(scope('a'))).into();
    let proof = Proof::capture(&source).unwrap();
    assert_eq!(proof.scope(), scope('a'));
    assert!(proof.is_for(&source.clone()));
    assert!(!proof.is_for(&source.to_vec().into()));
    assert!(Proof::capture(&br#"{"stickers":[],"links":[]}"#.to_vec().into()).is_none());
    assert!(Proof::capture(&document(serde_json::Value::Null).into()).is_none());
    assert_eq!(
        serde_json::to_string(&proof.scope()).unwrap(),
        format!("\"{}\"", "a".repeat(64))
    );
}
#[cfg(windows)]
#[test]
fn generated_scope_is_nonzero_bounded_and_not_tied_to_reused_unit_id() {
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..64 {
        let generated = Scope::generate().unwrap();
        let key = generated.to_string();
        assert_eq!(key.len(), 64);
        assert!(seen.insert(key.clone()));
        assert_eq!(
            serde_json::from_value::<Scope>(key.into()).unwrap(),
            generated
        );
    }
}
