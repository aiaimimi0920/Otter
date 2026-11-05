//! 写前意图只用 linshi 独占目录；异常/成功均保现场，不读取或删除真实 profile。
use super::*;
#[path = "projection_receive_cleanup_tests.rs"]
mod cleanup;
use serde_json::{json, Value};
pub(crate) struct Directory(pub PathBuf);
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-receive-journal-tests";
impl Directory {
    pub(crate) fn new() -> Self {
        let root = Path::new(ROOT);
        std::fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(crate) fn path(&self) -> PathBuf {
        self.0.join("projection-receive.json")
    }
}
fn invitation(index: usize) -> Invitation {
    let mut value = crate::projection_delivery::tests::invitation("https://loom.example", false);
    value["envelope"]["projectionId"] = format!("projection:{index:032x}").into();
    Inbox::parse(
        &serde_json::to_vec(&json!({"invitations":[value]})).unwrap(),
        "https://loom.example",
    )
    .unwrap()
    .into_entries()
    .pop()
    .unwrap()
}
use crate::projection_delivery::Inbox;
#[test]
fn begin_is_durable_idempotent_across_restart_and_tickets_share_one_store() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let first = store.begin(&invitation(1), "device-1").unwrap();
    let original = std::fs::read(directory.path()).unwrap();
    let mut newer = invitation(1);
    newer.revision = 10;
    newer.digest = "c".repeat(64);
    let retry = store.clone().begin(&newer, "device-1").unwrap();
    assert_eq!(retry.invitation().revision(), 4);
    assert_eq!(retry.invitation().digest(), "b".repeat(64));
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    let second = store.clone().begin(&invitation(2), "device-1").unwrap();
    assert_eq!(store.pending().unwrap().len(), 2);
    assert_eq!(Store::open(&directory.0).err(), Some(Error::Lock));
    assert!(!format!("{first:?}").contains(&"s".repeat(86)));
    drop((store, first, retry, second));
    let restarted = Store::open(&directory.0).unwrap();
    let pending = restarted.pending().unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].device(), "device-1");
    assert_eq!(pending[0].unit(), format!("delivery-{:032x}", 1));
    let bytes = std::fs::read(directory.path()).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    for forbidden in ["fixture-secret", "token", "imageBase64", "pixels"] {
        assert!(!text.contains(forbidden));
    }
    assert!(!bytes.starts_with(&[239, 187, 191]));
}
#[test]
fn retire_commits_removal_and_dropped_ticket_does_not_change_disk() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&invitation(1), "device-1").unwrap();
    let original = std::fs::read(directory.path()).unwrap();
    drop(ticket);
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    store.pending().unwrap().pop().unwrap().retire().unwrap();
    assert!(store.pending().unwrap().is_empty());
    drop(store);
    assert!(Store::open(&directory.0)
        .unwrap()
        .pending()
        .unwrap()
        .is_empty());
}
#[test]
fn external_modification_blocks_new_intent_retry_and_retirement_without_overwriting() {
    for mutation in [
        b"external writer".as_slice(),
        br#"{"version":1,"entries":[]}"#.as_slice(),
    ] {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let ticket = store.begin(&invitation(1), "device-1").unwrap();
        std::fs::write(directory.path(), mutation).unwrap();
        assert_eq!(
            store.begin(&invitation(1), "device-1").err(),
            Some(Error::Changed)
        );
        assert_eq!(
            store.begin(&invitation(2), "device-1").err(),
            Some(Error::Changed)
        );
        assert_eq!(ticket.retire(), Err(Error::Changed));
        assert_eq!(std::fs::read(directory.path()).unwrap(), mutation);
    }
}
#[test]
fn same_key_cannot_change_device_or_envelope_and_full_queue_is_preserved() {
    let directory = Directory::new();
    let entries = (0..MAX_ENTRIES)
        .map(|i| Record::new(&invitation(i), "device-1").unwrap())
        .collect();
    let bytes = serde_json::to_vec(&Document::new(entries)).unwrap();
    std::fs::write(directory.path(), &bytes).unwrap();
    let store = Store::open(&directory.0).unwrap();
    assert_eq!(
        store.begin(&invitation(MAX_ENTRIES), "device-1").err(),
        Some(Error::Limit)
    );
    assert_eq!(
        store.begin(&invitation(1), "device-2").err(),
        Some(Error::Changed)
    );
    let mut changed = invitation(1);
    let mut envelope = serde_json::to_value(changed.envelope()).unwrap();
    envelope["nonce"] = "3".repeat(32).into();
    changed.envelope = serde_json::from_value(envelope).unwrap();
    assert_eq!(
        store.begin(&changed, "device-1").err(),
        Some(Error::Changed)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    assert_eq!(store.pending().unwrap().len(), MAX_ENTRIES);
}
#[test]
fn roots_leases_and_ancestors_are_pinned_without_truncating_files() {
    let directory = Directory::new();
    assert_eq!(Store::open(Path::new("relative")).err(), Some(Error::Path));
    assert_eq!(
        Store::open(&directory.0.join("..")).err(),
        Some(Error::Path)
    );
    let sentinel = directory.0.join("keep");
    std::fs::write(&sentinel, b"keep").unwrap();
    assert_eq!(Store::open(&sentinel).err(), Some(Error::Path));
    let store = Store::open(&directory.0).unwrap();
    assert!(std::fs::rename(&directory.0, directory.0.with_extension("moved")).is_err());
    assert!(std::fs::rename(
        directory.0.join("projection-receive.lock"),
        directory.0.join("other.lock")
    )
    .is_err());
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
    drop(store);
}
#[test]
fn unique_valid_recovery_is_restored_without_deleting_it_and_ambiguity_fails_closed() {
    let directory = Directory::new();
    let bytes = serde_json::to_vec(&Document::new(vec![Record::new(
        &invitation(1),
        "device-1",
    )
    .unwrap()]))
    .unwrap();
    let recovery = directory
        .0
        .join("projection-receive.json.otter-recovery-00000000-0000-0000-0000-000000000001.json");
    std::fs::write(&recovery, &bytes).unwrap();
    let store = Store::open(&directory.0).unwrap();
    assert_eq!(store.pending().unwrap().len(), 1);
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    assert_eq!(std::fs::read(&recovery).unwrap(), bytes);
    drop(store);
    // 模拟规范路径丢失；两个可识别副本必须拒绝，而不是猜最新或回退为空。
    std::fs::remove_file(directory.path()).unwrap();
    let other = directory
        .0
        .join("projection-receive.json.otter-recovery-00000000-0000-0000-0000-000000000002.json");
    std::fs::write(&other, &bytes).unwrap();
    assert_eq!(Store::open(&directory.0).err(), Some(Error::Read));
    assert!(!directory.path().exists());
    assert_eq!(std::fs::read(recovery).unwrap(), bytes);
    assert_eq!(std::fs::read(other).unwrap(), bytes);
}
#[test]
fn invalid_oversized_or_directory_documents_never_become_an_empty_queue() {
    for bytes in [
        br#"{"version":2,"entries":[]}"#.to_vec(),
        vec![b' '; MAX_BYTES + 1],
        b"broken".to_vec(),
    ] {
        let directory = Directory::new();
        std::fs::write(directory.path(), &bytes).unwrap();
        assert!(Store::open(&directory.0).is_err());
        assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    }
    let directory = Directory::new();
    std::fs::create_dir(directory.path()).unwrap();
    assert!(Store::open(&directory.0).is_err());
}
#[test]
fn malformed_duplicate_and_unknown_record_fields_are_rejected_as_a_whole() {
    let good = serde_json::to_value(Document::new(vec![
        Record::new(&invitation(1), "device-1").unwrap()
    ]))
    .unwrap();
    for (key, value) in [
        ("intentId", json!("bad")),
        ("device", json!("../bad device")),
        ("unit", json!("unit-other")),
        ("expectedRevision", json!(0)),
        ("expectedDigest", json!("bad")),
        ("token", json!("never")),
    ] {
        let mut bad = good.clone();
        bad["entries"][0][key] = value;
        assert!(
            Document::decode(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{key}"
        );
    }
    let mut duplicated = good.clone();
    duplicated["entries"]
        .as_array_mut()
        .unwrap()
        .push(good["entries"][0].clone());
    assert!(Document::decode(&serde_json::to_vec(&duplicated).unwrap()).is_err());
    let mut wrong = good;
    wrong["extra"] = Value::Bool(true);
    assert!(Document::decode(&serde_json::to_vec(&wrong).unwrap()).is_err());
    assert!(Document::decode(br#"{"version":1,"version":1,"entries":[]}"#).is_err());
}
#[test]
fn committed_recovery_warning_keeps_the_new_baseline_and_does_not_claim_rollback() {
    let document = Document::new(vec![Record::new(&invitation(1), "device-1").unwrap()]);
    let bytes = crate::session_write::encode_bounded(&document, &AtomicBool::new(false)).unwrap();
    let mut state = State {
        original: None,
        entries: Vec::new(),
    };
    assert_eq!(
        state.published(bytes.clone(), document, true),
        Err(Error::RecoveryRetained)
    );
    assert!(state.original.as_ref().unwrap() == &bytes);
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].unit, format!("delivery-{:032x}", 1));
}
#[test]
fn lease_process_probe() {
    let Some(root) = std::env::var_os("OTTER_NATIVE_RECEIVE_JOURNAL_LEASE_TEST") else {
        return;
    };
    let root = PathBuf::from(root);
    assert_eq!(root.parent(), Some(Path::new(ROOT)));
    if std::env::var_os("OTTER_NATIVE_RECEIVE_JOURNAL_EXIT_TEST").is_some() {
        let store = Store::open(&root).unwrap();
        store.begin(&invitation(1), "device-1").unwrap();
        std::process::exit(81); // 故意跳过全部 Drop；真实 OS 回收句柄，持久意图仍需恢复。
    }
    assert_eq!(Store::open(&root).err(), Some(Error::Lock));
}
#[test]
fn process_exclusivity_and_abrupt_exit_preserve_the_durable_intent() {
    use std::{process::Command, time::Duration};
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "projection_delivery::journal::tests::lease_process_probe",
            "--nocapture",
        ])
        .env("OTTER_NATIVE_RECEIVE_JOURNAL_LEASE_TEST", &directory.0);
    assert!(
        crate::test_process::run(&mut command, Duration::from_secs(15))
            .unwrap()
            .status
            .success()
    );
    drop(store);
    command.env("OTTER_NATIVE_RECEIVE_JOURNAL_EXIT_TEST", "1");
    assert_eq!(
        crate::test_process::run(&mut command, Duration::from_secs(15))
            .unwrap()
            .status
            .code(),
        Some(81)
    );
    assert_eq!(
        Store::open(&directory.0).unwrap().pending().unwrap().len(),
        1
    );
}
