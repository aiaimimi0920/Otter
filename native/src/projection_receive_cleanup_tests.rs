//! 清理标记兼容旧记录、严格 CAS/许可、重启和减权幂等；不假造远端解绑。
use super::*;

#[test]
fn projection_cleanup_old_record_defaults_live_and_mark_is_durable_across_restart() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&invitation(1), "device-1").unwrap();
    let before: Value = serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    assert!(before["entries"][0].get("cleanup").is_none());
    assert!(!ticket.cleanup_pending());
    let marked = ticket.queue_cleanup().unwrap();
    assert!(marked.cleanup_pending());
    let after: Value = serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    assert_eq!(after["entries"][0]["cleanup"], true);
    for key in [
        "intentId",
        "envelope",
        "expectedRevision",
        "expectedDigest",
        "device",
        "unit",
    ] {
        assert_eq!(before["entries"][0][key], after["entries"][0][key]);
    }
    drop((marked, store));
    let store = Store::open(&directory.0).unwrap();
    assert!(store.pending().unwrap()[0].cleanup_pending());
    assert_eq!(
        store.begin(&invitation(1), "device-1").err(),
        Some(Error::Changed)
    );
}
#[test]
fn projection_cleanup_old_snapshot_can_only_mark_idempotently_not_reactivate_accept() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let mut old = store.begin(&invitation(1), "device-1").unwrap();
    drop(old.retry().queue_cleanup().unwrap());
    let bytes = std::fs::read(directory.path()).unwrap();
    assert_eq!(old.activate(), Err(Error::Changed));
    let marked = old.queue_cleanup().unwrap();
    assert!(marked.cleanup_pending());
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
}
#[test]
fn projection_cleanup_active_receive_permit_blocks_mark_without_disk_changes() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let mut held = store.begin(&invitation(1), "device-1").unwrap();
    held.activate().unwrap();
    let bytes = std::fs::read(directory.path()).unwrap();
    assert_eq!(held.retry().queue_cleanup().err(), Some(Error::Busy));
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    let retry = held.retry();
    drop(held);
    assert!(retry.queue_cleanup().unwrap().cleanup_pending());
}
#[test]
fn projection_cleanup_external_writer_is_not_overwritten_or_retired() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&invitation(1), "device-1").unwrap();
    std::fs::write(directory.path(), b"external writer").unwrap();
    assert_eq!(ticket.queue_cleanup().err(), Some(Error::Changed));
    assert_eq!(std::fs::read(directory.path()).unwrap(), b"external writer");
    assert!(!store.pending().unwrap()[0].cleanup_pending());
}
#[test]
fn projection_cleanup_invalid_flag_is_rejected_without_falling_back_to_live() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    drop(store.begin(&invitation(1), "device-1").unwrap());
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    drop(store);
    for bad in [json!("true"), json!(null), json!(1), json!({})] {
        value["entries"][0]["cleanup"] = bad;
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(directory.path(), &bytes).unwrap();
        assert_eq!(Store::open(&directory.0).err(), Some(Error::Format));
        assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    }
}
