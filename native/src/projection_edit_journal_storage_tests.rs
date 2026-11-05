//! 真实文件CAS/损坏/恢复边界；所有文件均由本测试在linshi内创建，不清理用户文件。
use super::*;
#[test]
fn external_rewrite_or_missing_file_rejects_ticket_without_erasing_local_pending() {
    for missing in [false, true] {
        let directory = Directory::new();
        let store = directory.store();
        let ticket = begin(&store);
        let original = std::fs::read(directory.path()).unwrap();
        if missing {
            std::fs::rename(directory.path(), directory.0.join("external-copy.json")).unwrap();
        } else {
            std::fs::write(directory.path(), b"{}").unwrap();
        }
        assert_eq!(ticket.check(&AtomicBool::new(false)), Err(Error::Changed));
        assert!(store.pending(&AtomicBool::new(false)).is_err());
        assert_eq!(
            ticket.commit_after_checkpoint(document(), &AtomicBool::new(false)),
            Err(Error::Changed)
        );
        if missing {
            assert_eq!(
                std::fs::read(directory.0.join("external-copy.json")).unwrap(),
                original
            );
        } else {
            assert_eq!(std::fs::read(directory.path()).unwrap(), b"{}");
        }
    }
}
#[test]
fn reopen_corruption_never_becomes_an_empty_store() {
    for bytes in [
        b"{}".as_slice(),
        br#"{"schemaVersion":1,"schemaVersion":1,"entries":[]}"#,
        b"{broken",
    ] {
        let directory = Directory::new();
        std::fs::write(directory.path(), bytes).unwrap();
        assert!(matches!(
            Store::open(&directory.0, &AtomicBool::new(false)),
            Err(Error::Format)
        ));
        assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    }
}
fn recovery(directory: &Directory, digit: char) -> PathBuf {
    directory.0.join(format!(
        "projection-edit.json.otter-recovery-{}-1111-1111-1111-111111111111.json",
        digit.to_string().repeat(8)
    ))
}
#[test]
fn unique_valid_recovery_is_reconstructed_without_removing_original_recovery_copy() {
    let directory = Directory::new();
    let store = directory.store();
    let ticket = begin(&store);
    let saved = std::fs::read(directory.path()).unwrap();
    drop((ticket, store));
    let backup = recovery(&directory, '1');
    std::fs::rename(directory.path(), &backup).unwrap();
    let reopened = directory.store();
    assert_eq!(reopened.pending(&AtomicBool::new(false)).unwrap().len(), 1);
    assert_eq!(std::fs::read(directory.path()).unwrap(), saved);
    assert_eq!(std::fs::read(backup).unwrap(), saved);
}
#[test]
fn corrupt_or_ambiguous_recovery_is_retained_and_not_published() {
    for ambiguous in [false, true] {
        let directory = Directory::new();
        let bytes = if ambiguous {
            serde_json::to_vec(&Ledger::new(Vec::new())).unwrap()
        } else {
            b"{}".to_vec()
        };
        std::fs::write(recovery(&directory, '1'), &bytes).unwrap();
        if ambiguous {
            std::fs::write(recovery(&directory, '2'), &bytes).unwrap();
        }
        assert!(Store::open(&directory.0, &AtomicBool::new(false)).is_err());
        assert!(!directory.path().exists());
        assert_eq!(std::fs::read(recovery(&directory, '1')).unwrap(), bytes);
    }
}
#[test]
fn oversized_or_nonregular_journal_fails_before_decode_and_survives() {
    let directory = Directory::new();
    let file = File::create(directory.path()).unwrap();
    file.set_len((MAX_BYTES + 1) as u64).unwrap();
    drop(file);
    assert!(matches!(
        Store::open(&directory.0, &AtomicBool::new(false)),
        Err(Error::Limit)
    ));
    assert_eq!(
        std::fs::metadata(directory.path()).unwrap().len(),
        (MAX_BYTES + 1) as u64
    );
    let directory = Directory::new();
    std::fs::create_dir(directory.path()).unwrap();
    assert!(Store::open(&directory.0, &AtomicBool::new(false)).is_err());
}
