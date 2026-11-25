use super::*;
use crate::desktop::loom_execution_tasks::reap_fixture::{wait_finished, Gate, WAIT};
use std::{sync::mpsc, time::Duration};
fn fixture() -> (PathBuf, PathBuf, Arc<Root>, Vec<u8>, Ready) {
    let directory = std::env::temp_dir().join(format!(
        "otter-archive-job-{}",
        crate::desktop::document_state::new_unit_id().unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let images = directory.join("images");
    std::fs::create_dir(&images).unwrap();
    let root = Arc::new(Root::open(&images).unwrap());
    let saved = br#"{"documentRevision":1,"stickers":[],"links":[],"future":true}"#.to_vec();
    let destination = directory.join("session.json");
    std::fs::write(&destination, &saved).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut prepare = super::super::Job::start(
        super::super::Input {
            saved: saved.clone(),
            revision: 1,
            root: None,
            catalog: Vec::new(),
        },
        Arc::new(move || {
            let _ = tx.send(());
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    prepare.test_wait_finished();
    (
        directory,
        destination,
        root,
        saved,
        prepare.poll().unwrap().unwrap(),
    )
}
#[test]
fn archive_worker_stages_without_commit_and_publishes_exact_next_revision() {
    let (directory, destination, root, saved, ready) = fixture();
    let gate = Gate::new(|| {});
    let mut job = Job::start(ready, root, destination.clone(), 123, gate.notify).unwrap();
    gate.noticed.recv_timeout(WAIT).unwrap();
    assert!(job.poll().is_none());
    assert!(job.deadline().is_some());
    gate.release.send(()).unwrap();
    wait_finished(job.worker.as_ref().unwrap());
    let staged = job.poll().unwrap().unwrap();
    assert!(job.poll().is_none());
    assert!(job.deadline().is_none());
    assert_eq!(std::fs::read(&destination).unwrap(), saved);
    assert_eq!(staged.candidate().revision, 2);
    let committed = staged.commit(&AtomicBool::new(false)).unwrap();
    assert!(committed.publication.retained_recovery.is_none());
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
    assert_eq!(document["documentRevision"], 2);
    assert_eq!(document["future"], true);
    assert_eq!(
        document["workflowAssetArchiveIndex"]["workflows"]["hook-live"]["updatedAt"],
        "123"
    );
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        committed.prepared.candidate().bytes.as_ref()
    );
    drop((committed, job));
    std::fs::remove_file(directory.join("session.json")).unwrap();
    std::fs::remove_dir(directory.join("images")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
#[test]
fn cancellation_or_changed_disk_never_overwrites_baseline() {
    for changed in [true, false] {
        let (directory, destination, root, saved, ready) = fixture();
        let expected = if changed {
            b"external writer".to_vec()
        } else {
            saved
        };
        std::fs::write(&destination, &expected).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut job = Job::start(
            ready,
            root,
            destination.clone(),
            123,
            Arc::new(move || {
                let _ = tx.send(());
            }),
        )
        .unwrap();
        rx.recv_timeout(Duration::from_secs(10)).unwrap();
        wait_finished(job.worker.as_ref().unwrap());
        if changed {
            assert!(job.poll().unwrap().is_err());
        } else {
            let staged = job.poll().unwrap().unwrap();
            assert!(staged.commit(&AtomicBool::new(true)).is_err());
        }
        drop(job);
        assert_eq!(std::fs::read(&destination).unwrap(), expected);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
        std::fs::remove_file(directory.join("session.json")).unwrap();
        std::fs::remove_dir(directory.join("images")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}

#[test]
fn archive_input_releases_snapshot_budget_but_retains_source_budget() {
    let (directory, _, root, saved, mut ready) = fixture();
    // 独立预算租约使生命周期断言不依赖全局计数或并行测试的图片分配。
    let source = saved.to_vec();
    let bytes = source.capacity();
    let budget = crate::byte_budget::Budget::new(bytes + 96);
    ready.prepared._leases.push(budget.reserve(96).unwrap());
    ready.source =
        crate::pixels::Pixels::from_reserved(source, budget.reserve(bytes).unwrap()).into();
    assert_eq!(budget.used(), bytes + 96);
    let input = ArchiveInput::from_ready(ready);
    assert_eq!(budget.used(), bytes);
    assert_eq!(input.source.as_slice(), saved.as_slice());
    assert!(input.metadata.matches(&input.source));
    drop(input);
    assert_eq!(budget.used(), 0);
    drop(root);
    std::fs::remove_file(directory.join("session.json")).unwrap();
    std::fs::remove_dir(directory.join("images")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
