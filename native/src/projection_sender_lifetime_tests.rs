//! 锁/并发/进程终态验证；测试子进程有界回收，故意退出后仍需同 ID 重开。
use super::*;
#[test]
fn roots_and_store_leases_are_pinned_and_foreign_intent_cannot_cancel() {
    let (directory, store) = setup();
    assert_eq!(
        Store::open(Path::new("relative"), &AtomicBool::new(false)).err(),
        Some(Error::Path)
    );
    assert_eq!(
        Store::open(&directory.0.join(".."), &AtomicBool::new(false)).err(),
        Some(Error::Path)
    );
    assert_eq!(
        Store::open(&directory.0, &AtomicBool::new(false)).err(),
        Some(Error::Lock)
    );
    assert!(std::fs::rename(&directory.0, directory.0.with_extension("moved")).is_err());
    assert!(std::fs::rename(
        directory.0.join("projection-sender.lock"),
        directory.0.join("other.lock")
    )
    .is_err());
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let (_, other) = setup();
    assert_eq!(other.cancel(&intent), Err(Error::Changed));
    drop(store);
    assert_eq!(
        Store::open(&directory.0, &AtomicBool::new(false)).err(),
        Some(Error::Lock)
    );
    drop(intent);
    directory.store();
}
#[test]
fn concurrent_same_key_has_one_durable_first_writer_and_shared_png_allocation() {
    let (directory, store) = setup();
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let store = store.clone();
            let directory = Directory(directory.0.clone());
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                prepare(&store, &directory, &selection(1)).unwrap()
            })
        })
        .collect();
    let intents: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    for intent in &intents {
        assert_eq!(intent.envelope(), intents[0].envelope());
        assert!(Arc::ptr_eq(
            &intent.record.prepared,
            &intents[0].record.prepared
        ));
    }
    assert_eq!(store.pending(&AtomicBool::new(false)).unwrap().len(), 1);
}
#[test]
fn committed_recovery_warning_keeps_new_baseline_without_claiming_rollback() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let bytes: Pixels = std::fs::read(directory.path()).unwrap().into();
    let mut state = State {
        original: None,
        entries: Vec::new(),
        retained: false,
    };
    assert_eq!(
        state.published(bytes.clone(), vec![intent.record.clone()], true),
        Err(Error::RecoveryRetained)
    );
    assert!(state.original.as_ref().unwrap() == &bytes);
    assert_eq!(state.entries.len(), 1);
    assert_eq!(
        store.baseline(&state, &AtomicBool::new(false)),
        Err(Error::RecoveryRetained)
    );
}
#[test]
fn process_probe() {
    let Some(root) = std::env::var_os("OTTER_NATIVE_SENDER_INTENT_TEST") else {
        return;
    };
    let directory = Directory(PathBuf::from(root));
    assert_eq!(directory.0.parent(), Some(Path::new(ROOT)));
    if std::env::var_os("OTTER_NATIVE_SENDER_EXIT_TEST").is_some() {
        let store = directory.store();
        prepare(&store, &directory, &selection(1)).unwrap();
        std::process::exit(82);
    }
    assert_eq!(
        Store::open(&directory.0, &AtomicBool::new(false)).err(),
        Some(Error::Lock)
    );
}
#[test]
fn actual_process_exclusivity_and_abrupt_exit_preserve_the_signed_intent() {
    use std::{process::Command, time::Duration};
    let (directory, store) = setup();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "projection_delivery::sender::tests::lifetime::process_probe",
            "--nocapture",
        ])
        .env("OTTER_NATIVE_SENDER_INTENT_TEST", &directory.0);
    assert!(
        crate::test_process::run(&mut command, Duration::from_secs(15))
            .unwrap()
            .status
            .success()
    );
    drop(store);
    command.env("OTTER_NATIVE_SENDER_EXIT_TEST", "1");
    assert_eq!(
        crate::test_process::run(&mut command, Duration::from_secs(15))
            .unwrap()
            .status
            .code(),
        Some(82)
    );
    let bytes = std::fs::read(directory.path()).unwrap();
    let reopened = directory.store();
    let intent = prepare(&reopened, &directory, &selection(1)).unwrap();
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    assert_eq!(
        reopened.pending(&AtomicBool::new(false)).unwrap()[0].envelope(),
        intent.envelope()
    );
}
