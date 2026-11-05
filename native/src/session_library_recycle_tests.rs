//! 关闭前候选验证：当前快照、拓扑裁剪、顺序/时间策略及硬预算；无窗口或磁盘副作用。
use super::*;
const SNAPSHOT: &[u8] = br#"{"stickers":[{"id":"a","type":"sticker","x":20,"y":30,"w":32,"h":40,"params":{},"inputs":[],"outputs":{},"extensionState":{"huge":123456789012345678901234567890}}],"links":[]}"#;
fn entry(id: &str, date: &str) -> serde_json::Value {
    serde_json::json!({"entryId":id,"sourceStickerId":"a","createdAt":date,"extension":{"kept":true},"snapshot":{"id":"a","x":0,"y":0,"w":10,"h":10}})
}
fn source(entries: Vec<serde_json::Value>) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"documentRevision":9,"stickers":[
        {"id":"a","x":0,"y":0,"w":10,"h":10},{"id":"b","x":0,"y":0,"w":10,"h":10},{"id":"c","x":0,"y":0,"w":10,"h":10}],
        "links":[{"id":"ab","fromUnitId":"a","fromPortId":"out","toUnitId":"b","toPortId":"in"},
        {"id":"bc","fromUnitId":"b","fromPortId":"out","toUnitId":"c","toPortId":"in"}],
        "recycleBin":entries,"referenceLibrary":42,"rootExtension":{"kept":true}})).unwrap()
}
fn run(original: &[u8], policy: policy::Policy) -> Result<Pixels, String> {
    prepare(
        original,
        Request {
            snapshot: SNAPSHOT,
            unit: &UnitId::parse("a").unwrap(),
            entry_id: "new",
            created_at: "1970-01-03T00:00:00.000Z",
        },
        policy,
        2 * 86_400_000,
        &AtomicBool::new(false),
    )
}
#[test]
fn current_snapshot_is_frozen_and_only_closing_window_and_its_links_are_removed() {
    let original = source(vec![entry("old", "legacy")]);
    let result = run(&original, policy::Policy::default()).unwrap();
    let document = session_document::decode(&result).unwrap();
    assert_eq!(document.revision, 9);
    assert_eq!(
        document
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        ["b", "c"]
    );
    assert_eq!(document.workspace.links().len(), 1);
    assert_eq!(document.workspace.links()[0].id.as_str(), "bc");
    let library = document.library(Kind::RecycleBin).unwrap();
    assert_eq!(library.entries().len(), 2);
    let next = library.find("new").unwrap();
    assert_eq!((next.window.w, next.window.h), (32.0, 40.0));
    let fields = candidate::fields(next.snapshot_json()).unwrap();
    assert!(!fields.contains_key("params") && !fields.contains_key("outputs"));
    assert_eq!(
        fields["extensionState"].get(),
        r#"{"huge":123456789012345678901234567890}"#
    );
    let root = candidate::fields(std::str::from_utf8(&result).unwrap()).unwrap();
    assert_eq!(root["referenceLibrary"].get(), "42");
    assert_eq!(
        session_document::decode(&original).unwrap().windows.len(),
        3
    );
}
#[test]
fn prune_is_age_then_tail_count_unknown_dates_and_cutoff_are_retained() {
    let original = source(vec![
        entry("expired", "1970-01-01T23:59:59.999Z"),
        entry("boundary", "1970-01-02T00:00:00Z"),
        entry("unknown", "legacy"),
    ]);
    let policy = policy::Policy {
        recycle_bin_max_entries: 0,
        recycle_bin_retention_days: 1,
    };
    let result = run(&original, policy).unwrap();
    let library = super::super::decode(&result, Kind::RecycleBin).unwrap();
    assert_eq!(
        library
            .entries()
            .iter()
            .map(|entry| entry.entry_id.as_ref())
            .collect::<Vec<_>>(),
        ["boundary", "unknown", "new"]
    );
    let result = run(
        &original,
        policy::Policy {
            recycle_bin_max_entries: 2,
            ..policy
        },
    )
    .unwrap();
    let library = super::super::decode(&result, Kind::RecycleBin).unwrap();
    assert_eq!(
        library
            .entries()
            .iter()
            .map(|entry| entry.entry_id.as_ref())
            .collect::<Vec<_>>(),
        ["unknown", "new"]
    );
    let result = run(
        &original,
        policy::Policy {
            recycle_bin_retention_days: 0,
            ..policy
        },
    )
    .unwrap();
    assert_eq!(
        super::super::decode(&result, Kind::RecycleBin)
            .unwrap()
            .entries()
            .len(),
        4
    );
}
#[test]
fn default_fifteen_keeps_recent_tail_and_unlimited_never_bypasses_hard_capacity() {
    let original = source(
        (0..20)
            .map(|i| entry(&format!("entry-{i}"), "legacy"))
            .collect(),
    );
    let result = run(&original, policy::Policy::default()).unwrap();
    let library = super::super::decode(&result, Kind::RecycleBin).unwrap();
    assert_eq!(library.entries().len(), 15);
    assert_eq!(library.entries()[0].entry_id, "entry-6");
    assert_eq!(library.entries().last().unwrap().entry_id, "new");
    let original = source(
        (0..MAX_ENTRIES)
            .map(|i| entry(&format!("entry-{i}"), "legacy"))
            .collect(),
    );
    assert!(run(
        &original,
        policy::Policy {
            recycle_bin_max_entries: 0,
            recycle_bin_retention_days: 0
        }
    )
    .is_err());
    assert_eq!(
        super::super::decode(
            &run(&original, policy::Policy::default()).unwrap(),
            Kind::RecycleBin
        )
        .unwrap()
        .entries()
        .len(),
        15
    );
}
#[test]
fn failed_preparation_cannot_close_or_overwrite_source() {
    let original = source(vec![entry("new", "legacy")]);
    assert!(run(&original, policy::Policy::default()).is_err());
    let original = source(Vec::new());
    let unit = UnitId::parse("a").unwrap();
    assert!(prepare(
        &original,
        Request {
            snapshot: SNAPSHOT,
            unit: &unit,
            entry_id: "new",
            created_at: "date"
        },
        policy::Policy::default(),
        0,
        &AtomicBool::new(true)
    )
    .is_err());
    assert!(run(
        &original,
        policy::Policy {
            recycle_bin_max_entries: 501,
            recycle_bin_retention_days: 0
        }
    )
    .is_err());
    let art = String::from_utf8(SNAPSHOT.to_vec())
        .unwrap()
        .replace("\"type\":\"sticker\"", "\"type\":\"art\"");
    assert!(prepare(
        &original,
        Request {
            snapshot: art.as_bytes(),
            unit: &unit,
            entry_id: "new",
            created_at: "date"
        },
        policy::Policy::default(),
        0,
        &AtomicBool::new(false)
    )
    .is_err());
    assert_eq!(
        session_document::decode(&original).unwrap().windows.len(),
        3
    );
}
#[test]
fn settings_read_policy_independently_and_preserve_it_when_updating_temp_cache() {
    let mut document = crate::app_settings_document::Document::decode(
        br#"{"cache":{"recycleBinMaxEntries":30,"recycleBinRetentionDays":7,"extension":true}}"#,
    )
    .unwrap();
    let expected = policy::Policy {
        recycle_bin_max_entries: 30,
        recycle_bin_retention_days: 7,
    };
    assert_eq!(document.recycle_policy().unwrap(), expected);
    document
        .set_cache_policy(crate::cache_policy::Policy::default())
        .unwrap();
    assert_eq!(
        crate::app_settings_document::Document::decode(&document.encode().unwrap())
            .unwrap()
            .recycle_policy()
            .unwrap(),
        expected
    );
    assert_eq!(
        crate::app_settings_document::Document::default()
            .recycle_policy()
            .unwrap(),
        policy::Policy::default()
    );
    assert!(crate::app_settings_document::Document::decode(
        br#"{"cache":{"recycleBinRetentionDays":3651}}"#
    )
    .unwrap()
    .recycle_policy()
    .is_err());
}
