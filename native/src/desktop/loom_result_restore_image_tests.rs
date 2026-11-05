//! 真实内联 PNG 恢复、像素复用、预览优先级和失败原子性。
use super::*;
use crate::{
    desktop::{
        image::Image,
        loom_result_state::{Results, Status},
    },
    workspace_id::UnitId,
};
fn image() -> Image {
    Image {
        width: 1,
        height: 1,
        pixels: vec![10, 20, 30, 255].into(),
    }
}
fn raw(value: serde_json::Value) -> Box<serde_json::value::RawValue> {
    serde_json::value::RawValue::from_string(value.to_string()).unwrap()
}
fn encoded() -> crate::loom_base64::Encoded {
    let cancel = AtomicBool::new(false);
    let png = crate::desktop::export::png_raw(&image(), &cancel).unwrap();
    crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap()
}
#[test]
fn actual_candidate_restores_shared_image_ports_and_idle_display() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let url = encoded();
    let cancel = AtomicBool::new(false);
    let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{
        "id":"art","type":"art","x":0,"y":0,"w":1,"h":1,
        "outputs":{"output":url.as_str(),"output_image":url.as_str(),"text":"hello"}
    }],"links":[]}))
    .unwrap();
    let candidate = crate::desktop::session_candidate::prepare(bytes, None, &cancel).unwrap();
    let results = candidate.prepare_results(&cancel).unwrap();
    let result = &results[0];
    let first = result.image_source("output").unwrap();
    let second = result.image_source("output_image").unwrap();
    assert!(first.image.pixels.same_storage(&second.image.pixels));
    assert_eq!(first.image.pixels.as_ref(), image().pixels.as_ref());
    assert!(result
        .display()
        .unwrap()
        .pixels
        .same_storage(&first.image.pixels));
    assert!(result.holds_restored_preview());
    assert_eq!(result.status(), Status::Idle);
    assert!(result.active().is_none() && result.surface_result_revision().is_none());
    assert_eq!(result.save_output_images().len(), 2);
    assert_eq!(result.save_value_outputs().unwrap()["text"], "hello");
}
#[test]
fn invalid_later_inline_image_or_cancellation_keeps_owner_unchanged() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let url = encoded();
    for cancelled in [false, true] {
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        let stamp = results.output_stamp();
        let input = raw(serde_json::json!({"a":url.as_str(),"z":"data:image/png;base64,invalid"}));
        assert!(results
            .restore_outputs(&input, &AtomicBool::new(cancelled))
            .is_err());
        assert!(stamp.matches(&results));
        assert!(results.ports().is_empty() && results.display().is_none());
        assert!(results.save_value_outputs().is_none());
    }
}
#[test]
fn explicit_preview_wins_and_external_references_stay_values() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let preview = Image {
        width: 1,
        height: 1,
        pixels: vec![40, 50, 60, 255].into(),
    };
    let mut results = Results::from_restored_display(
        UnitId::parse("node:test").unwrap(),
        Some(preview.clone()),
        true,
    );
    let url = encoded();
    results
        .restore_outputs(
            &raw(serde_json::json!({"output_image":url.as_str(),
                "network":"https://invalid.test/a.png", "path":"C:/private.png",
                "descriptor":{"uri":"https://invalid.test/b.png"}
            })),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(results
        .display()
        .unwrap()
        .pixels
        .same_storage(&preview.pixels));
    assert!(results.image_source("output_image").is_some());
    assert!(results.image_source("network").is_none());
    assert_eq!(results.save_value_outputs().unwrap().len(), 3);
}
