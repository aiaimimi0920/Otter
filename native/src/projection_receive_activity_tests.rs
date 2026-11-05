//! 同 key 单一消费许可；跨 key 保并行、未完成/旧 Ticket 不能抢先解绑，Drop 无磁盘写入。
use super::*;
use crate::projection_delivery::{accept::tests::invitation, journal::tests::Directory};
const ORIGIN: &str = "https://loom.example";
#[test]
fn permit_is_exclusive_until_ticket_drop_and_does_not_rewrite_the_journal() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let mut first = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    first.activate().unwrap();
    first.activate().unwrap(); // 同一移动 owner 可沿 Ready 移交补偿，不产生第二许可。
    let mut competing = store.pending().unwrap().pop().unwrap();
    assert_eq!(competing.activate(), Err(Error::Busy));
    assert_eq!(
        store
            .begin(&invitation(ORIGIN), "device-1")
            .unwrap()
            .retire(),
        Err(Error::Busy)
    );
    let before = std::fs::read(directory.path()).unwrap();
    drop(first);
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    competing.activate().unwrap();
    competing.retire().unwrap();
    assert!(store.pending().unwrap().is_empty());
}
#[test]
fn retired_ticket_is_rejected_before_a_new_intent_can_be_touched() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let first = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    let mut stale = store.pending().unwrap().pop().unwrap();
    first.retire().unwrap();
    let mut replacement = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    let bytes = std::fs::read(directory.path()).unwrap();
    assert_eq!(stale.activate(), Err(Error::Changed));
    replacement.activate().unwrap();
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
}
#[test]
fn distinct_projection_keys_can_hold_independent_permits() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let mut first = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    let mut second = store
        .begin(&invitation("https://other.example"), "device-1")
        .unwrap();
    first.activate().unwrap();
    second.activate().unwrap();
    assert_eq!(store.pending().unwrap().len(), 2);
}
#[test]
fn retry_snapshot_preserves_record_but_never_duplicates_the_active_permit() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let mut first = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    first.activate().unwrap();
    let mut retry = first.retry();
    assert!(first.same_record(&retry));
    assert_eq!(retry.activate(), Err(Error::Busy));
    first.retire().unwrap();
    assert_eq!(retry.activate(), Err(Error::Changed));
    let replacement = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    assert!(!replacement.same_record(&retry));
}
