//! 批次容量包括旧关联与primary；失败发生在Store/网络之前，不扩大原8项上限。
use super::*;
use serde_json::{json, Value};
fn selections(count: usize) -> Vec<Selection> {
    (0..count)
        .map(|index| {
            Selection::new(
                &"a".repeat(64),
                "https://loom.example",
                "unit-1",
                &format!("new-{index}"),
            )
            .unwrap()
        })
        .collect()
}
fn source() -> Source {
    serde_json::to_vec(
        &json!({"stickers":[{"id":"unit-1","x":0,"y":0,"w":4,"h":4}],
        "links":[],"otterNativeProjectionScope":"a".repeat(64)}),
    )
    .unwrap()
    .into()
}
#[test]
fn batch_admission_rejects_empty_duplicate_mixed_identity_and_over_capacity_without_writes() {
    let source = source();
    let before = source.to_vec();
    assert!(Binding::admit_batch(&source, &[]).is_err());
    assert!(Binding::admit_batch(&source, &selections(MAX_PER_UNIT)).is_ok());
    assert_eq!(
        Binding::admit_batch(&source, &selections(MAX_PER_UNIT + 1)),
        Err(Error::Limit)
    );
    let mut duplicate = selections(2);
    duplicate[1] = duplicate[0].clone();
    assert!(Binding::admit_batch(&source, &duplicate).is_err());
    for selection in [
        Selection::new(&"b".repeat(64), "https://loom.example", "unit-1", "other").unwrap(),
        Selection::new(&"a".repeat(64), "https://other.example", "unit-1", "other").unwrap(),
        Selection::new(&"a".repeat(64), "https://loom.example", "unit-2", "other").unwrap(),
    ] {
        assert!(Binding::admit_batch(&source, &[selections(1).remove(0), selection]).is_err());
    }
    assert_eq!(&*source, &before);
}
#[test]
fn batch_admission_counts_existing_and_primary_and_preserves_unknown_fields() {
    let original = source();
    let (_directory, _store, intent) =
        super::super::tests::signed("unit-1", &"a".repeat(64), "https://loom.example");
    let candidate = super::super::tests::candidate(&original, &intent, 1, intent.digest());
    let mut value: Value = serde_json::from_slice(candidate.source()).unwrap();
    value["futureRoot"] = json!({"untouched":true});
    value["stickers"][0]["qrProjection"] =
        value["stickers"][0]["projectionSenders"][0]["link"].clone();
    let source: Source = serde_json::to_vec(&value).unwrap().into();
    assert!(
        Binding::admit_batch(&source, &selections(7)).is_ok(),
        "same primary signature counts once"
    );
    assert_eq!(
        Binding::admit_batch(&source, &selections(8)),
        Err(Error::Limit)
    );
    let existing = Selection::new(
        &"a".repeat(64),
        "https://loom.example",
        "unit-1",
        intent.target(),
    )
    .unwrap();
    assert!(Binding::admit_batch(&source, &[existing]).is_err());
    value["stickers"][0]["qrProjection"]["role"] = "receiver".into();
    assert!(
        Binding::admit_batch(&serde_json::to_vec(&value).unwrap().into(), &selections(1)).is_err()
    );
}
