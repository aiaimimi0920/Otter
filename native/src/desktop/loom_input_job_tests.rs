//! 真实后台 WIC 输入准备及 owner 回收；不访问网络或用户数据。
use super::*;
use crate::{editor::Tool, viewport::Point};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
pub(crate) static SERIAL: Mutex<()> = Mutex::new(());
#[test]
fn completion_notification_allows_immediate_poll_after_permit_release() {
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let mut job = Job::default();
    let (send, receive) = std::sync::mpsc::channel();
    job.start_with_notify(
        id("target"),
        &editor,
        vec![source("image", "source", &editor)],
        Arc::new(move || {
            send.send(ACTIVE.load(Ordering::Acquire)).unwrap();
        }),
    )
    .unwrap();
    assert_eq!(receive.recv_timeout(Duration::from_secs(10)).unwrap(), 0);
    let batch = job
        .poll()
        .expect("completion notification must make poll ready")
        .unwrap();
    assert_eq!(batch.descriptors().len(), 1);
    assert!(!job.is_busy());
    assert!(job.poll().is_none());
}
#[test]
fn freshness_survives_input_release_and_rejects_each_changed_dependency() {
    let target = Editor::default();
    let mut input = Editor::default();
    let batch = batch::prepare(
        id("target"),
        target.document_stamp(),
        vec![source("image", "input", &input)],
        &AtomicBool::new(false),
    )
    .unwrap();
    let freshness = batch.freshness().unwrap();
    drop(batch);
    let check = |editor: &Editor, input_editor: Option<&Editor>| {
        freshness.is_current(|unit, stamp| {
            let current = if *unit == id("target") {
                Some(editor)
            } else {
                input_editor
            };
            current.is_some_and(|current| stamp.matches(current) && current.draft().is_none())
        })
    };
    assert!(check(&target, Some(&input)));
    assert!(!check(&Editor::default(), Some(&input)));
    assert!(!check(&target, None));
    assert!(!check(&target, Some(&Editor::default())));
    input.begin(Tool::Brush, Point { x: 0.5, y: 0.5 }).unwrap();
    assert!(!check(&target, Some(&input)));
    input.commit();
    assert!(!check(&target, Some(&input)));
}
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn source(port: &str, unit: &str, editor: &Editor) -> Source {
    Source::capture(
        port.into(),
        id(unit),
        1,
        1,
        vec![0, 0, 255, 255].into(),
        editor,
    )
    .unwrap()
}
fn finish(job: &mut Job) -> Result<PreparedBatch, Error> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = job.poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "Loom input worker timeout");
        thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn batch_worker_preserves_order_and_rejects_edited_replaced_or_closed_sources() {
    let _serial = SERIAL.lock().unwrap();
    let target = Editor::default();
    let mut original = Editor::default();
    let mut job = Job::default();
    job.start(
        id("target"),
        &target,
        vec![
            source("image", "source", &original),
            source("mask", "source", &original),
        ],
    )
    .unwrap();
    assert!(matches!(
        job.start(id("target"), &target, Vec::new()),
        Err(Error::Busy)
    ));
    let ready = finish(&mut job).unwrap();
    assert_eq!(
        ready
            .descriptors()
            .map(|(port, _)| port)
            .collect::<Vec<_>>(),
        ["image", "mask"]
    );
    assert!(ready.is_current(|unit| if *unit == id("target") {
        Some(&target)
    } else {
        Some(&original)
    }));
    assert!(!ready.is_current(|unit| if *unit == id("target") {
        Some(&target)
    } else {
        None
    }));
    let replaced = Editor::default();
    assert!(!ready.is_current(|unit| if *unit == id("target") {
        Some(&target)
    } else {
        Some(&replaced)
    }));
    original
        .begin(Tool::Brush, Point { x: 0.5, y: 0.5 })
        .unwrap();
    original.commit();
    assert!(!ready.is_current(|unit| if *unit == id("target") {
        Some(&target)
    } else {
        Some(&original)
    }));
    assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
}
#[test]
fn completed_then_cancelled_candidate_is_not_delivered_and_job_is_reusable() {
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let mut job = Job::default();
    job.start(
        id("target"),
        &editor,
        vec![source("image", "source", &editor)],
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !job.running.as_ref().unwrap().worker.is_finished() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    job.cancel();
    assert!(matches!(job.poll(), Some(Err(Error::Cancelled))));
    job.start(id("target"), &editor, vec![]).unwrap();
    assert_eq!(finish(&mut job).unwrap().descriptors().len(), 0);
}
#[test]
fn target_close_replacement_and_uncommitted_edit_reject_prepared_batch() {
    let _serial = SERIAL.lock().unwrap();
    let mut target = Editor::default();
    let mut job = Job::default();
    job.start(id("target"), &target, vec![]).unwrap();
    let ready = finish(&mut job).unwrap();
    assert!(ready.is_current(|_| Some(&target)));
    assert!(!ready.is_current(|_| None));
    let replacement = Editor::default();
    assert!(!ready.is_current(|_| Some(&replacement)));
    target.begin(Tool::Brush, Point { x: 0.5, y: 0.5 }).unwrap();
    assert!(!ready.is_current(|_| Some(&target)));
    target.commit();
    assert!(!ready.is_current(|_| Some(&target)));
}
#[test]
fn dropping_active_job_joins_and_does_not_retain_snapshot_pixels() {
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let pixels: Pixels = vec![0, 0, 0, 255].into();
    let weak = pixels.downgrade();
    let input = Source::capture("image".into(), id("source"), 1, 1, pixels, &editor).unwrap();
    let mut job = Job::default();
    job.start(id("target"), &editor, vec![input]).unwrap();
    drop(job);
    assert!(weak.upgrade().is_none());
    assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
}
#[test]
fn worker_and_batch_admission_fail_without_starting_jobs() {
    let _serial = SERIAL.lock().unwrap();
    let editor = Editor::default();
    let mut job = Job::default();
    assert!(matches!(
        job.start(
            id("target"),
            &editor,
            vec![source("same", "a", &editor), source("same", "b", &editor)]
        ),
        Err(Error::InvalidInput)
    ));
    let inputs = (0..33)
        .map(|index| source(&index.to_string(), "a", &editor))
        .collect();
    assert!(matches!(
        job.start(id("target"), &editor, inputs),
        Err(Error::InvalidInput)
    ));
    let first = reserve_worker().unwrap();
    let second = reserve_worker().unwrap();
    assert!(matches!(
        job.start(id("target"), &editor, vec![]),
        Err(Error::WorkerLimit)
    ));
    assert!(!job.is_busy());
    drop((first, second));
    assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
}
