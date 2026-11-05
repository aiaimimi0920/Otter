//! 真实恢复链保留原 sourceSize 与原文身份，不把当前非等比 Editor 框写成自然源映射。
use crate::desktop::{session_binding::Bound, session_candidate, Apartment};
use std::sync::atomic::AtomicBool;

#[test]
fn restored_nonuniform_crop_retains_original_mapping_and_shared_source_baseline() {
    let _apartment = Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let image = crate::desktop::image::Image {
        width: 16,
        height: 16,
        pixels: [10, 20, 30, 255].repeat(256).into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let json = serde_json::json!({"stickers":[{
        "id":"s", "x":0, "y":0, "w":8, "h":4, "src":url.as_str(),
        "imageEditState":{"sourceSize":{"w":8,"h":8},
            "cropRect":{"x":2,"y":2,"w":4,"h":4}, "flippedX":true}
    }], "links":[]});
    let candidate =
        session_candidate::prepare(serde_json::to_vec(&json).unwrap(), None, &cancel).unwrap();
    let record = candidate.retained_source_record();
    let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
    let bound = Bound::prepare_for(vectors, None, &cancel).ok().unwrap();
    let prepared = bound
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    let mut canvas = prepared.states[0].canvas.borrow_mut();
    let frame = canvas.editor.full_logical_frame().unwrap();
    assert_eq!((frame.width, frame.height), (16.0, 8.0));
    let expected = crate::crop_geometry::Size {
        width: 8.0,
        height: 8.0,
    };
    let source = canvas.minified_source.as_ref().unwrap();
    assert_eq!(source.source_logical(&canvas.editor), Some(expected));
    let origin = source.origin(&canvas.editor).unwrap();
    assert!(origin.record.same_storage(&record));
    assert_eq!(origin.unit.as_str(), "s");
    assert_eq!((source.image().width, source.image().height), (16, 16));
    assert!(canvas.editor.reset_crop());
    let source = canvas.minified_source.as_ref().unwrap();
    assert_eq!(source.source_logical(&canvas.editor), Some(expected));
    assert!(source
        .origin(&canvas.editor)
        .unwrap()
        .record
        .same_storage(&record));
    assert!(canvas.editor.crop_view().is_none());
}

#[test]
fn restored_legacy_view_history_and_save_keep_the_full_natural_source() {
    let _apartment = Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let image = crate::desktop::image::Image {
        width: 8,
        height: 8,
        pixels: (0..64)
            .flat_map(|i| [(i % 8) as u8, (i / 8) as u8, 0, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let json = serde_json::json!({"stickers":[{
        "id":"s", "x":0, "y":0, "w":8, "h":8, "src":url.as_str(),
        "imageEditState":{"cropRect":{"x":2,"y":0,"w":2,"h":8},"flippedX":true}
    }],"links":[]});
    let candidate =
        session_candidate::prepare(serde_json::to_vec(&json).unwrap(), None, &cancel).unwrap();
    let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
    let prepared = Bound::prepare_for(vectors, None, &cancel)
        .ok()
        .unwrap()
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    let mut canvas = prepared.states[0].canvas.borrow_mut();
    let base = canvas.editor.layers().unwrap().base.clone();
    let expected = crate::crop_geometry::Rect {
        x: 4.0,
        y: 0.0,
        width: 2.0,
        height: 8.0,
    };
    assert_eq!(
        canvas.editor.crop_view().unwrap().legacy_pixel_region(),
        Some(expected)
    );
    assert_eq!(
        canvas.minified_source.as_ref().unwrap().image().pixels[0],
        7
    );
    assert!(canvas
        .minified_source
        .as_ref()
        .unwrap()
        .matches(&canvas.editor));
    let saved = crate::session_crop_write::merge_view(
        Some(serde_json::json!({
            "cropRect":{"future":42},"sourceSize":{"w":100,"h":100},"borderWidth":3
        })),
        canvas.editor.crop_view(),
    )
    .unwrap();
    assert_eq!(saved["cropRect"]["x"], 4.0);
    assert_eq!(saved["cropRect"]["future"], 42);
    assert_eq!(saved["borderWidth"], 3);
    assert!(saved.get("sourceSize").is_none());
    assert!(canvas.editor.reset_crop());
    let reset =
        crate::session_crop_write::merge_view(Some(saved.clone()), canvas.editor.crop_view())
            .unwrap();
    assert!(reset.get("cropRect").is_none());
    assert!(canvas.editor.layers().unwrap().base.same_storage(&base));
    assert!(canvas
        .minified_source
        .as_ref()
        .unwrap()
        .matches(&canvas.editor));
    assert!(canvas.editor.undo());
    let undone =
        crate::session_crop_write::merge_view(Some(reset), canvas.editor.crop_view()).unwrap();
    assert_eq!(undone["cropRect"]["x"], 4.0);
    assert_eq!(undone["cropRect"]["w"], 2.0);
    assert!(undone.get("sourceSize").is_none());
    assert!(canvas.editor.redo());
    assert!(canvas.editor.crop_view().is_none());
}
