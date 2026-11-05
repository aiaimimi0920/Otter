//! 完成通知和候选所有权回归，不创建真实工作区窗口。
use super::*;
use std::{sync::mpsc, time::Duration};
fn bytes() -> Vec<u8> {
    br#"{"documentSchemaVersion":1,"documentRevision":7,"stickers":[{"id":"old","x":-50,"y":10,"w":200,"h":100,"opacityNormal":0,"params":{"future":"keep"},"minified":true,"savedRect":{"x":0,"y":0,"w":800,"h":600},"cropOffset":{"x":25,"y":15}}],"links":[],"future":{"keep":true}}"#.to_vec()
}
#[test]
fn completed_candidate_owns_original_and_window_metadata() {
    let original = bytes();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        original.clone(),
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let prepared = job.poll().unwrap().unwrap();
    assert_eq!(prepared.vectors.len(), 1);
    let candidate = prepared.candidate;
    assert!(job.poll().is_none());
    drop(job);
    assert_eq!(candidate.original_json(), original);
    assert_eq!(candidate.revision, 7);
    assert_eq!(candidate.placements[0].unit.as_str(), "old");
    assert_eq!(candidate.placements[0].opacity_normal, 0.0);
    assert_eq!(candidate.placements[0].rect.x, -50.0);
    let expanded = candidate.placements[0].expanded_rect().unwrap();
    assert_eq!(
        (expanded.x, expanded.y, expanded.w, expanded.h),
        (-75.0, -5.0, 800.0, 600.0)
    );
    assert!(candidate.images[0].display().is_none());
}
#[test]
fn worker_prepares_real_text_and_blur_before_publishing_candidate() {
    let input = super::super::loom_inline::prepare(
        2,
        2,
        vec![255; 16].into(),
        &crate::editor::Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("PNG")
    };
    let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{"id":"u","x":0,"y":0,"w":8,"h":4,
        "src":format!("data:image/png;base64,{base64}"),"annotationState":{"serialCounter":0,"elements":[
            {"id":"text","type":"text","zIndex":0,"x":1,"y":2,"text":"恢复测试","style":{"width":1,"color":"#fff"}},
            {"id":"blur","type":"blur","zIndex":1,"points":[{"x":1,"y":1}],"brushWidth":8,"strength":4.4,"style":{"width":1,"color":"#fff"}}
        ]}}],"links":[]})).unwrap();
    let caller = std::thread::current().id();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        bytes,
        None,
        Arc::new(move || {
            tx.send(std::thread::current().id()).unwrap();
        }),
    )
    .unwrap();
    assert_ne!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), caller);
    let prepared = job.poll().unwrap().unwrap();
    let elements = prepared.vectors[0].1.editor.elements();
    assert_eq!(elements[0].text.as_ref().unwrap().content(), "恢复测试");
    let blur = elements[1].blur.as_ref().unwrap();
    assert_eq!((blur.width(), blur.height(), blur.strength()), (8, 4, 4.4));
    let weak = blur.pixels().downgrade();
    drop(prepared);
    drop(job);
    assert!(weak.upgrade().is_none());
}
#[test]
fn unsupported_annotation_fails_job_instead_of_publishing_partial_restore() {
    let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{"id":"u","x":0,"y":0,"w":8,"h":4,
        "annotationState":{"serialCounter":0,"elements":[{"id":"bad","type":"future","zIndex":0}]}}],"links":[]})).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        bytes,
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(job.poll().unwrap().is_err());
    assert!(job.poll().is_none());
}
#[test]
fn cancellation_after_completion_still_discards_unpublished_candidate() {
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        bytes(),
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    job.cancel();
    assert!(job.poll().unwrap().is_err());
    assert!(job.poll().is_none());
}
#[test]
fn invalid_session_completes_with_error_and_drop_reaps_worker() {
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        b"invalid".to_vec(),
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(job.poll().unwrap().is_err());
    let (tx, rx) = mpsc::channel();
    let job = Job::start(
        bytes(),
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    drop(job);
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
}
#[test]
fn completion_notification_follows_asset_directory_handle_release() {
    let directory =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-session-job-{}",
            super::super::document_state::new_unit_id()
                .unwrap()
                .as_str()
        ));
    std::fs::create_dir(&directory).unwrap();
    let root = Root::open(&directory).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        bytes(),
        Some(root),
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    std::fs::remove_dir(directory).unwrap();
    assert!(job.poll().unwrap().is_ok());
}
