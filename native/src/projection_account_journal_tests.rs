//! Write-ahead records, locks, cancellation, and conflicts on real isolated files; no existing profiles or network replay.
use super::*;
use journal::{Intent, Store};

fn directory() -> std::path::PathBuf {
    let root = std::path::Path::new(
        "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-account-journal-tests",
    );
    std::fs::create_dir_all(root).unwrap();
    let path = root.join(format!("{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir(&path).unwrap();
    path
}
fn intent() -> Intent {
    Intent {
        envelope: envelope(&image(17)),
        unit: "receive-1".into(),
        scope: serde_json::from_value(json!("a".repeat(64))).unwrap(),
        loom_origin: "https://127.0.0.1:54443".into(),
        device: RECEIVER.into(),
    }
}
#[test]
fn journal_reopen_exact_retry_and_retire_preserve_durable_identity_and_exclusive_lock() {
    let root = directory();
    let cancel = AtomicBool::new(false);
    let intent = intent();
    let mut store = Store::open(&root, &cancel).unwrap();
    assert!(Store::open(&root, &cancel).is_err());
    store.begin(intent.clone(), &cancel).unwrap();
    let original = std::fs::read(root.join("projection-account.json")).unwrap();
    store.begin(intent.clone(), &cancel).unwrap();
    assert_eq!(
        std::fs::read(root.join("projection-account.json")).unwrap(),
        original
    );
    drop(store);
    let mut reopened = Store::open(&root, &cancel).unwrap();
    assert!(reopened.entries() == [intent.clone()]);
    reopened.retire(&intent, &cancel).unwrap();
    drop(reopened);
    assert!(Store::open(&root, &cancel).unwrap().entries().is_empty());
}
#[test]
fn journal_rejects_second_invitation_for_same_unit_before_persisting_an_unreadable_ledger() {
    let root = directory();
    let cancel = AtomicBool::new(false);
    let first = intent();
    let mut second = first.clone();
    second.envelope = envelope(&image(211));
    let mut store = Store::open(&root, &cancel).unwrap();
    store.begin(first.clone(), &cancel).unwrap();
    let original = std::fs::read(root.join("projection-account.json")).unwrap();
    assert!(matches!(store.begin(second, &cancel), Err(Error::Identity)));
    assert_eq!(
        std::fs::read(root.join("projection-account.json")).unwrap(),
        original
    );
    drop(store);
    assert!(Store::open(&root, &cancel).unwrap().entries() == [first]);
}
#[test]
fn journal_cancel_and_external_file_change_never_mutate_accepted_intent_or_replace_sentinel() {
    let root = directory();
    let cancel = AtomicBool::new(false);
    let intent = intent();
    let mut store = Store::open(&root, &cancel).unwrap();
    assert!(store.begin(intent.clone(), &AtomicBool::new(true)).is_err());
    assert!(store.entries().is_empty() && !root.join("projection-account.json").exists());
    store.begin(intent.clone(), &cancel).unwrap();
    let original = std::fs::read(root.join("projection-account.json")).unwrap();
    assert!(store.retire(&intent, &AtomicBool::new(true)).is_err());
    assert_eq!(
        std::fs::read(root.join("projection-account.json")).unwrap(),
        original
    );
    std::fs::write(root.join("projection-account.json"), b"external sentinel").unwrap();
    assert!(store.retire(&intent, &cancel).is_err());
    assert!(store.entries() == [intent]);
    assert_eq!(
        std::fs::read(root.join("projection-account.json")).unwrap(),
        b"external sentinel"
    );
}
