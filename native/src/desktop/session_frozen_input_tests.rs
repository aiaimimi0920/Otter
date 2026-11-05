//! 冻结 worker 的跨线程、资源准入、取消和输入预算回归；只使用内嵌图片。
use super::*;
use crate::desktop::session_job::{self, Job};
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

fn input(src: &str) -> Input {
    Input {
        source: serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],"referenceLibrary":[{
            "entryId":"entry","sourceStickerId":"old","createdAt":"legacy",
            "snapshot":{"id":"old","src":src,"x":0,"y":0,"w":32,"h":32,"extension":{"keep":true}}
        }]})).unwrap().into(),
        kind: Kind::ReferenceLibrary, entry_id:"entry".into(), unit:UnitId::parse("fresh").unwrap(), mouse:(-20.0,30.0),
    }
}

#[test]
fn frozen_image_is_prepared_on_worker_without_changing_source() {
    let png = crate::loom_base64::encode_png_data_url(
        include_bytes!("../../../src-tauri/icons/32x32.png"),
        &AtomicBool::new(false),
    )
    .unwrap();
    let input = input(png.as_str());
    let source = input.source.clone();
    let before = source.as_ref().to_vec();
    let (tx, rx) = mpsc::channel();
    let caller = std::thread::current().id();
    let mut job = Job::start_input(
        session_job::Input::Frozen(input),
        None,
        Arc::new(move || {
            tx.send(std::thread::current().id()).unwrap();
        }),
    )
    .unwrap();
    assert_ne!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), caller);
    let ready = job.poll().unwrap().unwrap();
    assert_eq!(ready.candidate.placements[0].unit.as_str(), "fresh");
    assert_eq!(ready.candidate.placements[0].rect.x, 30.0);
    assert!(ready.candidate.origin.is_none());
    let image = ready.candidate.images[0].display().unwrap();
    assert_eq!((image.width, image.height), (32, 32));
    assert!(std::str::from_utf8(ready.candidate.original_json())
        .unwrap()
        .contains("extension"));
    assert_eq!(source.as_ref(), before);
    assert!(job.poll().is_none());
}

#[test]
fn frozen_file_reference_is_not_an_implicit_resource_permission() {
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_input(
        session_job::Input::Frozen(input("Z:/not-authorized.png")),
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(job.poll().unwrap().is_err());
}

#[test]
fn cancelling_frozen_worker_returns_no_candidate_and_drop_releases_source() {
    let input = input("");
    let Source::Imported(bytes) = &input.source else {
        panic!("fixture source")
    };
    let weak = Arc::downgrade(bytes);
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_input(
        session_job::Input::Frozen(input),
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    )
    .unwrap();
    job.cancel();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(job.poll().unwrap().is_err());
    drop(job);
    assert!(weak.upgrade().is_none());
}

#[test]
fn malformed_or_oversized_input_is_rejected_before_spawning() {
    let mut bad = input("");
    bad.entry_id = "".into();
    assert!(Job::start_input(
        session_job::Input::Frozen(bad),
        None,
        Arc::new(|| panic!("must not start"))
    )
    .is_err());
    let mut bad = input("");
    bad.mouse.0 = f64::NAN;
    assert!(Job::start_input(
        session_job::Input::Frozen(bad),
        None,
        Arc::new(|| panic!("must not start"))
    )
    .is_err());
    let mut bad = input("");
    bad.source = vec![b' '; crate::session_document::MAX_SESSION_BYTES + 1].into();
    assert!(Job::start_input(
        session_job::Input::Frozen(bad),
        None,
        Arc::new(|| panic!("must not start"))
    )
    .is_err());
}
