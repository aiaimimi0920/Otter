//! 冻结实例化准备的身份、原文、坐标和取消合同；不发布窗口或访问用户文件。
use super::*;

#[test]
fn field_admission_rejects_duplicates_and_overflow_without_losing_escaped_keys() {
    assert!(fields(r#"{"a":1,"a":2}"#).is_err());
    assert_eq!(
        fields(r#"{"\u0061": { "raw" : true }}"#).unwrap()["a"].get(),
        r#"{ "raw" : true }"#
    );
    let mut map = serde_json::Map::new();
    for index in 0..256 {
        map.insert(format!("field-{index}"), true.into());
    }
    assert!(fields(&serde_json::to_string(&map).unwrap()).is_ok());
    map.insert("overflow".into(), true.into());
    assert!(fields(&serde_json::to_string(&map).unwrap()).is_err());
}

#[test]
fn prepared_snapshot_enters_existing_worker_without_granting_file_access() {
    use crate::desktop::session_job::Job;
    use std::{
        sync::{mpsc, Arc},
        time::Duration,
    };
    for file_reference in [false, true] {
        let mut value: serde_json::Value = serde_json::from_slice(&source()).unwrap();
        let snapshot = &mut value["recycleBin"][0]["snapshot"];
        snapshot.as_object_mut().unwrap().remove("filePath");
        snapshot["src"] = if file_reference {
            "Z:/never-open-this.png".into()
        } else {
            "".into()
        };
        let bytes = serde_json::to_vec(&value).unwrap();
        let prepared = run(&bytes, "worker-copy", (10.0, 20.0), false).unwrap();
        let (tx, rx) = mpsc::channel();
        let mut job = Job::start(
            prepared.bytes.as_ref().to_vec(),
            None,
            Arc::new(move || {
                tx.send(()).unwrap();
            }),
        )
        .unwrap();
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let result = job.poll().unwrap();
        if file_reference {
            assert!(result.is_err());
        } else {
            let ready = result.unwrap();
            assert_eq!(ready.candidate.placements.len(), 1);
            assert_eq!(ready.candidate.placements[0].unit, prepared.unit);
            assert_eq!(ready.candidate.placements[0].rect.x, 60.0);
            assert_eq!(ready.vectors.len(), 1);
            assert!(ready.candidate.images[0].display().is_none());
        }
        assert!(job.poll().is_none());
        assert_eq!(
            crate::session_document::decode(&bytes)
                .unwrap()
                .library(Kind::RecycleBin)
                .unwrap()
                .entries()
                .len(),
            1
        );
    }
}

#[test]
fn minified_copy_keeps_saved_frame_and_crop_contract() {
    let mut value: serde_json::Value = serde_json::from_slice(&source()).unwrap();
    let snapshot = &mut value["recycleBin"][0]["snapshot"];
    snapshot["minified"] = true.into();
    snapshot["savedRect"] = serde_json::json!({"x":1,"y":2,"w":200,"h":100});
    snapshot["cropOffset"] = serde_json::json!({"x":5,"y":10});
    let prepared = run(
        &serde_json::to_vec(&value).unwrap(),
        "mini-copy",
        (0.0, 0.0),
        false,
    )
    .unwrap();
    let document = crate::session_document::decode(prepared.bytes.as_ref()).unwrap();
    let window = &document.windows[0];
    assert!(window.is_minified());
    assert_eq!(window.saved_rect.unwrap().w, 200.0);
    assert_eq!(window.crop_offset.unwrap().x, 5.0);
    let expanded = window.expanded_rect().unwrap();
    assert_eq!(
        (expanded.x, expanded.y, expanded.w, expanded.h),
        (45.0, 40.0, 200.0, 100.0)
    );
}

fn source() -> Vec<u8> {
    br#"{"stickers":[{"id":"active","x":0,"y":0,"w":10,"h":10}],"links":[],
      "otterNativeCoordinateSpace":{"version":1,"origin":[-100,20],"scale":1.5,"future":true},
      "recycleBin":[{"entryId":"entry","sourceStickerId":"old","createdAt":"legacy",
      "snapshot":{"id":"old","x":1,"y":2,"w":80,"h":60,"src":"",
        "groupId":"not-live","outputs":{"secret":"not-executed"},"executionConfig":{},
        "params":{"stale":true},"opacityNormal":0.6,"opacityMini":0.8,
        "extensionState": { "number":123456789012345678901234567890, "keep" : true },
        "ocrResult":{"text":"retained"},"filePath":"Z:/not-authorized.png"}}]}"#
        .to_vec()
}
fn run(bytes: &[u8], id: &str, mouse: (f64, f64), cancel: bool) -> Result<Prepared, String> {
    prepare(
        &crate::session_document::decode(bytes).unwrap(),
        Kind::RecycleBin,
        "entry",
        UnitId::parse(id).unwrap(),
        mouse,
        &AtomicBool::new(cancel),
    )
}

#[test]
fn fresh_instance_preserves_raw_extensions_but_not_runtime_ownership() {
    let original = source();
    let saved = original.clone();
    let prepared = run(&original, "fresh", (-100.0, 200.0), false).unwrap();
    assert_eq!(original, saved);
    let document = crate::session_document::decode(prepared.bytes.as_ref()).unwrap();
    assert_eq!(document.windows.len(), 1);
    let window = &document.windows[0];
    assert_eq!(window.id, "fresh");
    assert_eq!(
        (window.x, window.y, window.w, window.h),
        (-50.0, 250.0, 80.0, 60.0)
    );
    assert_eq!(window.opacity(false), 0.6);
    assert!(window.group_id.is_none());
    assert!(window.outputs.is_none());
    assert!(window.execution_config.is_none());
    assert_eq!(window.params.unwrap().get(), "{}");
    assert_eq!(window.file_path.as_deref(), Some("Z:/not-authorized.png"));
    assert_eq!(document.coordinate_space.unwrap().scale, 1.5);
    let text = std::str::from_utf8(prepared.bytes.as_ref()).unwrap();
    assert!(text.contains(r#"{ "number":123456789012345678901234567890, "keep" : true }"#));
    assert!(text.contains("retained"));
    assert!(document
        .library(Kind::RecycleBin)
        .unwrap()
        .entries()
        .is_empty());
    assert_eq!(
        crate::session_document::decode(&original)
            .unwrap()
            .library(Kind::RecycleBin)
            .unwrap()
            .entries()
            .len(),
        1
    );
}

#[test]
fn collisions_invalid_position_and_cancel_never_consume_entry() {
    let bytes = source();
    for id in ["old", "active"] {
        assert!(run(&bytes, id, (0.0, 0.0), false).is_err());
    }
    for mouse in [(f64::NAN, 0.0), (0.0, f64::INFINITY), (f64::MAX, 0.0)] {
        assert!(run(&bytes, "fresh", mouse, false).is_err());
    }
    assert!(run(&bytes, "fresh", (0.0, 0.0), true).is_err());
    assert_eq!(bytes, source());
}

#[test]
fn reference_can_be_copied_repeatedly_and_missing_entry_fails() {
    let bytes = String::from_utf8(source())
        .unwrap()
        .replace("recycleBin", "referenceLibrary");
    let document = crate::session_document::decode(bytes.as_bytes()).unwrap();
    for id in ["copy-a", "copy-b"] {
        let copy = prepare(
            &document,
            Kind::ReferenceLibrary,
            "entry",
            UnitId::parse(id).unwrap(),
            (0.0, 0.0),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(copy.unit.as_str(), id);
    }
    assert!(prepare(
        &document,
        Kind::ReferenceLibrary,
        "absent",
        UnitId::parse("fresh").unwrap(),
        (0.0, 0.0),
        &AtomicBool::new(false)
    )
    .is_err());
    assert_eq!(
        document
            .library(Kind::ReferenceLibrary)
            .unwrap()
            .entries()
            .len(),
        1
    );
}
