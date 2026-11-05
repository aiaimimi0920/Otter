//! 裁剪只影响回收数组；无变化、时间/数量顺序、取消与原文边界。
use super::*;
fn input() -> Vec<u8> {
    let entries: Vec<_> = ["1970-01-01T00:00:00Z", "legacy", "1970-01-02T00:00:00Z", "1970-01-03T00:00:00Z"]
        .iter().enumerate().map(|(i, date)| serde_json::json!({"entryId":format!("e-{i}"),"sourceStickerId":"a",
            "createdAt":date,"extension":{"keep":true},"snapshot":{"id":"a","x":0,"y":0,"w":10,"h":10}})).collect();
    serde_json::to_vec(
        &serde_json::json!({"documentSchemaVersion":1,"documentRevision":9,"stickers":[],"links":[],
        "recycleBin":entries,"referenceLibrary":42,"extension":{"n":123}}),
    )
    .unwrap()
}
#[test]
fn age_then_tail_count_preserves_unknown_dates_other_library_and_revision() {
    let original = input();
    let policy = Policy {
        recycle_bin_max_entries: 2,
        recycle_bin_retention_days: 1,
    };
    let bytes = prepare(&original, policy, 2 * 86_400_000, &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["recycleBin"].as_array().unwrap().len(), 2);
    assert_eq!(value["recycleBin"][0]["entryId"], "e-2");
    assert_eq!(value["recycleBin"][1]["entryId"], "e-3");
    assert_eq!(value["recycleBin"][0]["extension"]["keep"], true);
    assert_eq!(value["documentRevision"], 9);
    assert_eq!(value["referenceLibrary"], 42);
    assert_eq!(value["extension"]["n"], 123);
    let policy = Policy {
        recycle_bin_max_entries: 0,
        recycle_bin_retention_days: 1,
    };
    let bytes = prepare(&original, policy, 2 * 86_400_000, &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    let library = super::super::decode(&bytes, Kind::RecycleBin).unwrap();
    assert_eq!(library.entries()[0].entry_id, "e-1");
    assert_eq!(library.entries().len(), 3);
}
#[test]
fn no_change_does_not_allocate_candidate_and_missing_array_remains_missing() {
    for bytes in [
        input(),
        br#"{"stickers":[],"links":[],"extension":42}"#.to_vec(),
    ] {
        assert!(
            prepare(&bytes, Policy::default(), 0, &AtomicBool::new(false))
                .unwrap()
                .is_none()
        );
        assert!(prepare(
            &bytes,
            Policy {
                recycle_bin_max_entries: 0,
                recycle_bin_retention_days: 0
            },
            i64::MAX,
            &AtomicBool::new(false)
        )
        .unwrap()
        .is_none());
    }
}
#[test]
fn cancellation_invalid_policy_and_invalid_library_leave_source_unchanged() {
    let bytes = input();
    let before = bytes.clone();
    assert!(prepare(&bytes, Policy::default(), 0, &AtomicBool::new(true)).is_err());
    assert!(prepare(
        &bytes,
        Policy {
            recycle_bin_max_entries: 501,
            recycle_bin_retention_days: 0
        },
        0,
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(prepare(
        br#"{"stickers":[],"links":[],"recycleBin":42}"#,
        Policy::default(),
        0,
        &AtomicBool::new(false)
    )
    .is_err());
    assert_eq!(bytes, before);
}
