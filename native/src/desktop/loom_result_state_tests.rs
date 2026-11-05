use super::*;
use crate::workspace_id::UnitId;
use serde_json::json;
use std::sync::atomic::AtomicBool;
fn request(generation: u64) -> Request {
    Request {
        id: UnitId::parse("request").unwrap(),
        generation,
    }
}
fn values(value: serde_json::Value) -> Batch {
    super::super::loom_output_batch::decode(&value, None, &AtomicBool::new(false)).unwrap()
}
#[test]
fn formal_source_stamp_ignores_previews_but_detects_successful_output_replacement() {
    let mut state = Results::new(UnitId::parse("unit").unwrap());
    let empty = state.output_stamp();
    let other = Results::new(UnitId::parse("unit").unwrap());
    assert!(!empty.matches(&other));
    state.begin(request(1));
    state.preview(&request(1), 1, image(10)).unwrap();
    assert!(empty.matches(&state));
    assert!(state.image_source("output_image").is_none());
    state.finish(&request(1), image(20)).unwrap();
    assert!(!empty.matches(&state));
    let source = state.image_source("output_image").unwrap();
    assert_eq!(source.image.pixels[0], 20);
    assert_eq!(state.display().unwrap().pixels[0], 10);
    assert!(source.stamp.matches(&state));
    assert_eq!(
        source.image.pixels.as_ptr(),
        state.image_source("output").unwrap().image.pixels.as_ptr()
    );
    state.begin(request(2));
    state.preview(&request(2), 1, image(30)).unwrap();
    assert!(source.stamp.matches(&state));
    assert!(state.finish(&request(1), image(40)).is_err());
    assert!(source.stamp.matches(&state));
    state.cancel(&request(2));
    assert!(source.stamp.matches(&state));
    state.begin(request(3));
    state.finish(&request(3), image(50)).unwrap();
    assert!(!source.stamp.matches(&state));
    // 下游快照仍安全持有原图，但交付前必须拒绝过期凭证。
    assert_eq!(source.image.pixels[0], 20);
    assert_eq!(
        state.image_source("output_image").unwrap().image.pixels[0],
        50
    );
}
#[test]
fn lifecycle_rejects_stale_progress_and_bounds_unicode_failure_without_losing_output() {
    let mut state = Results::new(UnitId::parse("unit").unwrap());
    assert_eq!(state.status(), Status::Idle);
    state.begin(request(1));
    assert_eq!(state.status(), Status::Running);
    assert!(state.set_progress(&request(1), 0.3));
    assert!(!state.set_progress(&request(1), f64::NAN));
    assert_eq!(state.progress(), Some(0.3));
    state
        .finish(
            &request(1),
            values(json!({"output":{"kind":"value","value":7}})),
        )
        .unwrap();
    assert_eq!(state.status(), Status::Completed);
    assert_eq!(state.progress(), Some(1.0));
    state.begin(request(2));
    assert!(!state.set_progress(&request(1), 0.9));
    assert!(!state.fail(&request(1), "old error"));
    assert!(state.fail(&request(2), &"错误🦦".repeat(1000)));
    assert_eq!(state.status(), Status::Failed);
    assert_eq!(state.error().unwrap().chars().count(), 1024);
    assert_eq!(state.ports().len(), 1);
    state.begin(request(3));
    assert!(state.error().is_none());
    assert!(state.progress().is_none());
    assert!(state.cancel(&request(3)));
    assert_eq!(state.status(), Status::Cancelled);
    assert_eq!(state.ports().len(), 1);
}
fn image(color: u8) -> Batch {
    let input = super::super::loom_inline::prepare(
        1,
        1,
        vec![color, 0, 0, 255].into(),
        &crate::editor::Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("inline");
    };
    values(
        json!({"output_image":{"kind":"inline_resource","mime":"image/png","dataBase64":base64}}),
    )
}
#[test]
fn preview_is_display_only_and_final_preserves_workflow_preview() {
    let mut state = Results::new(UnitId::parse("unit").unwrap());
    state.begin(request(1));
    assert!(state.preview(&request(1), 2, image(20)).unwrap());
    assert!(!state.preview(&request(1), 1, image(10)).unwrap());
    assert!(state.ports().is_empty());
    let preview = state.display().unwrap().pixels.as_ptr();
    state.finish(&request(1), image(30)).unwrap();
    assert_eq!(state.display().unwrap().pixels.as_ptr(), preview);
    assert_eq!(state.display().unwrap().pixels[0], 20);
    assert_eq!(state.ports().len(), 2);
    let mut pointers = Vec::new();
    for port in state.ports() {
        let Output::Image(image) = &port.value else {
            panic!("image");
        };
        assert_eq!(image.pixels[0], 30);
        pointers.push(image.pixels.as_ptr());
    }
    assert_eq!(pointers[0], pointers[1]);
    state.begin(request(2));
    state.finish(&request(2), image(40)).unwrap();
    assert_eq!(state.display().unwrap().pixels[0], 40);
}
#[test]
fn finals_merge_ports_preserve_null_and_reject_old_generations() {
    let mut state = Results::new(UnitId::parse("unit").unwrap());
    state.begin(request(1));
    state
        .finish(
            &request(1),
            values(
                json!({"old":{"kind":"value","value":7},"output":{"kind":"value","value":"first"}}),
            ),
        )
        .unwrap();
    state.begin(request(2));
    assert_eq!(
        state.finish(
            &request(1),
            values(json!({"output":{"kind":"value","value":"stale"}}))
        ),
        Err(Error::Stale)
    );
    state
        .finish(
            &request(2),
            values(json!({"output":{"kind":"value","value":null}})),
        )
        .unwrap();
    assert_eq!(state.ports().len(), 2);
    assert!(state
        .ports()
        .iter()
        .any(|port| matches!(&port.value, Output::Value(value) if value.is_null())));
    assert!(state.active().is_none());
}
#[test]
fn overflow_is_atomic_and_old_cancel_cannot_end_new_execution() {
    let mut state = Results::new(UnitId::parse("unit").unwrap());
    state.begin(request(1));
    let mut ports = serde_json::Map::new();
    for index in 0..128 {
        ports.insert(
            format!("port{index}"),
            json!({"kind":"value","value":index}),
        );
    }
    state.finish(&request(1), values(ports.into())).unwrap();
    state.begin(request(2));
    assert_eq!(
        state.finish(
            &request(2),
            values(json!({"new":{"kind":"value","value":1}}))
        ),
        Err(Error::Capacity)
    );
    assert_eq!(state.ports().len(), 128);
    assert_eq!(state.active(), Some(&request(2)));
    assert!(!state.cancel(&request(1)));
    assert!(state.cancel(&request(2)));
}
