//! Completion-notification and candidate-ownership regressions without creating actual workspace windows.
use super::*;
use crate::desktop::loom_execution_tasks::reap_fixture::{Gate, WAIT};
use std::{sync::mpsc, time::Duration};
fn bytes() -> Vec<u8> {
    br#"{"documentSchemaVersion":1,"documentRevision":7,"stickers":[{"id":"old","x":-50,"y":10,"w":200,"h":100,"opacityNormal":0,"params":{"future":"keep"},"minified":true,"savedRect":{"x":0,"y":0,"w":800,"h":600},"cropOffset":{"x":25,"y":15}}],"links":[],"future":{"keep":true}}"#.to_vec()
}
#[test]
fn completed_candidate_owns_original_and_window_metadata() {
    let original = bytes();
    let gate = Gate::new(|| {});
    let mut job = Job::start(original.clone(), None, gate.notify).unwrap();
    gate.noticed.recv_timeout(WAIT).unwrap();
    assert!(job.poll().is_none());
    assert!(job.deadline().is_some());
    gate.release.send(()).unwrap();
    job.test_wait_finished();
    let prepared = job.poll().unwrap().unwrap().restore.vector;
    assert_eq!(prepared.vectors.len(), 1);
    let candidate = prepared.candidate;
    assert!(job.poll().is_none());
    assert!(job.deadline().is_none());
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
    job.test_wait_finished();
    let prepared = job.poll().unwrap().unwrap().restore.vector;
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
    job.test_wait_finished();
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
    job.test_wait_finished();
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
    job.test_wait_finished();
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
    let directory = std::env::temp_dir().join(format!(
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
    job.test_wait_finished();
    assert!(job.poll().unwrap().is_ok());
}

#[test]
fn worker_resolves_catalog_and_materializes_formal_ports_before_completion() {
    use crate::session_art_batch::Binding;
    let cancel = AtomicBool::new(false);
    let png = include_bytes!("../../assets/branding/32x32.png");
    let url = crate::loom_base64::encode_png_data_url(png, &cancel).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[
        {"id":"art","type":"art","artId":"known","x":0,"y":0,"w":32,"h":32,
         "outputs":{"output_image":url.as_str(),"alias":url.as_str(),"text":"hello"},"params":{"value":42}},
        {"id":"pin","x":48,"y":0,"w":16,"h":16},
        {"id":"missing","type":"art","artId":"not-loaded","x":32,"y":0,"w":16,"h":16}
    ],"links":[]})).unwrap();
    let mut handshake = crate::loom_handshake::tests::fixture();
    handshake["capabilities"]["artDefinitions"] = serde_json::json!([
        {"id":"known","parameters":[{"id":"value"}],"inputs":[],"outputs":[]}
    ]);
    let catalog = crate::loom_handshake::response(&serde_json::to_vec(&handshake).unwrap())
        .unwrap()
        .unwrap();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_input(
        Input::Bytes(bytes),
        None,
        Some(catalog),
        Arc::new(move || {
            tx.send(std::thread::current().id()).unwrap();
        }),
    )
    .unwrap();
    assert_ne!(
        rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        std::thread::current().id()
    );
    job.test_wait_finished();
    let ready = job.poll().unwrap().unwrap();
    assert!(ready.frozen.is_none());
    let ready = ready.restore;
    assert_eq!(
        ready
            .vector
            .vectors
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["art", "pin", "missing"]
    );
    assert_eq!(ready.arts.arts.len(), 2);
    let Binding::Ready {
        capability_index: 0,
        restored,
    } = &ready.arts.arts[0].binding
    else {
        panic!("known capability not resolved");
    };
    assert!(restored.parameters.get("value").is_some());
    assert!(
        matches!(&ready.arts.arts[1].binding, Binding::Missing { art_id: Some(id) } if id == "not-loaded")
    );
    assert_eq!(
        ready
            .arts
            .results
            .iter()
            .map(|result| result.unit().as_str())
            .collect::<Vec<_>>(),
        ["art", "missing"]
    );
    let result = &ready.arts.results[0];
    let image = result.image_source("output_image").unwrap().image;
    assert_eq!((image.width, image.height), (32, 32));
    assert_eq!(image.encoded_png().unwrap().as_ref(), png);
    assert!(image
        .pixels
        .same_storage(&result.image_source("alias").unwrap().image.pixels));
    assert_eq!(result.save_value_outputs().unwrap()["text"], "hello");
    assert!(result.active().is_none());
}

#[test]
fn worker_rejects_corrupt_formal_image_after_valid_output_without_returning_a_batch() {
    let bytes = br#"{"stickers":[
        {"id":"first","type":"art","x":0,"y":0,"w":2,"h":2,"outputs":{"value":1}},
        {"id":"second","type":"art","x":0,"y":0,"w":2,"h":2,"outputs":{"bad":"data:image/png;base64,invalid"}}
    ],"links":[]}"#.to_vec();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start(bytes, None, Arc::new(move || tx.send(()).unwrap())).unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    job.test_wait_finished();
    let error = job
        .poll()
        .unwrap()
        .err()
        .expect("invalid formal output reached UI");
    assert!(error.contains("Invalid saved outputs"), "{error}");
    assert!(job.poll().is_none());
}
