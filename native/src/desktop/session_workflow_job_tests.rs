//! 实际 worker 贯通与生命周期验证；通知并非保存或发送授权。
use super::*;
use serde_json::json;
use std::{
    sync::{atomic::AtomicUsize, mpsc},
    time::Duration,
};
fn input() -> Input {
    Input {
        saved: br#"{"documentRevision":5,"stickers":[],"links":[]}"#.to_vec(),
        revision: 5,
        root: None,
        catalog: Vec::new(),
    }
}
fn run(input: Input) -> std::result::Result<Ready, String> {
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        input,
        Arc::new(move || {
            let _ = tx.send(());
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(30)).unwrap();
    let result = job.poll().unwrap();
    assert!(job.poll().is_none());
    result
}
#[test]
fn worker_prepares_actual_image_snapshot_metadata_and_source_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let image = crate::desktop::image::Image {
        width: 8,
        height: 4,
        pixels: [10, 20, 30, 255].repeat(32).into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let saved = serde_json::to_vec(&json!({"documentRevision":5,"stickers":[{
        "id":"image","x":0,"y":0,"w":4,"h":2,"src":url.as_str(),"opacityNormal":0.5,
        "originWorkflowId":"remote","originNodeId":"node"}],"links":[]}))
    .unwrap();
    let ready = run(Input {
        saved: saved.clone(),
        ..input()
    })
    .unwrap();
    assert!(ready.matches_source(&saved));
    assert!(!ready.matches_source(b"{}"));
    assert_eq!(ready.snapshot().revision(), 5);
    let decoded = crate::desktop::session_image::inline(
        ready.snapshot().value()["nodes"][0]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap();
    assert_eq!((decoded.width, decoded.height), (8, 4));
    assert!((127..=128).contains(&decoded.pixels[3]));
    assert_eq!(ready.components(&cancel).unwrap().len(), 1);
    assert!(ready.metadata().matches(&saved));
}
#[test]
fn worker_rejects_revision_mismatch_and_unadmitted_file_access() {
    assert!(run(Input {
        revision: 6,
        ..input()
    })
    .is_err());
    let mut document = json!({"documentRevision":5,"stickers":[{
        "id":"file","x":0,"y":0,"w":1,"h":1,"filePath":"C:\\not-admitted.png"}],"links":[]});
    // filePath 只是保留的原文件元数据，不应自动触发读取。
    assert!(run(Input {
        saved: serde_json::to_vec(&document).unwrap(),
        ..input()
    })
    .is_ok());
    document["stickers"][0]["src"] = json!("C:\\not-admitted.png");
    assert!(run(Input {
        saved: serde_json::to_vec(&document).unwrap(),
        ..input()
    })
    .is_err());
}
#[test]
fn shared_source_is_copied_on_worker_and_retains_its_budget() {
    let source: crate::session_source::Source = input().saved.into();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_shared(
        source.clone(),
        None,
        Vec::new(),
        Arc::new(move || {
            let _ = tx.send(());
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(30)).unwrap();
    let ready = job.poll().unwrap().unwrap();
    assert!(ready.matches_source(&source));
    assert!(ready._source_lease.as_ref().unwrap().bytes() >= source.len());
    assert_ne!(ready.source.as_ptr(), source.as_ptr());
    drop(source);
    assert_eq!(ready.snapshot().revision(), 5);
}

#[test]
fn completed_result_is_discarded_after_cancel_and_notification_is_once() {
    let count = Arc::new(AtomicUsize::new(0));
    let notified = count.clone();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(
        input(),
        Arc::new(move || {
            notified.fetch_add(1, Ordering::AcqRel);
            let _ = tx.send(());
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(30)).unwrap();
    job.cancel();
    assert!(job.poll().unwrap().is_err());
    assert!(job.poll().is_none());
    assert_eq!(count.load(Ordering::Acquire), 1);
}
#[test]
fn drop_cancels_and_joins_worker_and_panic_becomes_failure() {
    let exited = Arc::new(AtomicBool::new(false));
    let done = exited.clone();
    let job = Job::start_with(Arc::new(|| {}), move |cancel| {
        while !cancel.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        done.store(true, Ordering::Release);
        Err("cancelled".into())
    })
    .unwrap();
    drop(job);
    assert!(exited.load(Ordering::Acquire));
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_with(
        Arc::new(move || {
            let _ = tx.send(());
        }),
        |_| panic!("test worker failure"),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(30)).unwrap();
    assert!(job.poll().unwrap().is_err());
}
