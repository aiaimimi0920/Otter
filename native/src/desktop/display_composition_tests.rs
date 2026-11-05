//! 内存夹具验证后台合成与既有导出一致；不读取用户图片、剪贴板或桌面。
use super::*;
use crate::{editor::Tool, erase_layers::Layers, viewport::Point};

fn image() -> Image {
    Image {
        width: 8,
        height: 8,
        pixels: vec![255; 256].into(),
    }
}

#[test]
fn reset_logical_frame_bypasses_shared_buffer_only_when_mapping_changes() {
    let image = image();
    for size in [7.25, 8.0] {
        let mut editor = Editor::with_layers(Layers {
            size: (8, 8),
            base: image.pixels.clone(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap()
        .with_restored_crop(crate::crop_geometry::Crop {
            rect: crate::crop_geometry::Rect {
                x: 1.0,
                y: 1.0,
                width: 2.0,
                height: 2.0,
            },
            source: crate::crop_geometry::Size {
                width: size,
                height: size,
            },
        })
        .unwrap();
        assert!(editor.reset_crop());
        let stamp = editor.document_stamp();
        let output = compose(&image, &editor, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            (output.width, output.height),
            (size.round() as u32, size.round() as u32)
        );
        assert_eq!(output.pixels.same_storage(&image.pixels), size == 8.0);
        assert!(stamp.matches(&editor));
        if size != 8.0 {
            let _apartment = Apartment::new().unwrap();
            let (png, _, _) = export::png_payload(&image, &editor).unwrap();
            let decoded = Image::from_encoded(&png).unwrap();
            assert_eq!(output.pixels, decoded.pixels);
        }
    }
}

#[test]
fn unchanged_image_shares_authoritative_pixels_without_composition() {
    let image = image();
    let other = Image {
        pixels: vec![0; 256].into(),
        ..image.clone()
    };
    let editor = Editor::with_layers(Layers {
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    let result = compose(&other, &editor, &AtomicBool::new(false)).unwrap();
    assert!(result.pixels.same_storage(&image.pixels));
    assert!(!result.pixels.same_storage(&other.pixels));
}

#[test]
fn worker_composes_crop_raster_and_vector_without_mutating_source() {
    let image = image();
    let mut editor = Editor::with_layers(Layers {
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: Some([0, 0, 255, 255].repeat(4).into()),
        annotation_size: Some((2, 2)),
    })
    .unwrap();
    editor.begin(Tool::Brush, Point { x: 3.0, y: 3.0 }).unwrap();
    assert!(editor.commit());
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let stamp = editor.document_stamp();
    let source = editor.layers().unwrap().clone();
    let worker_image = image.clone();
    let snapshot = editor.export_snapshot();
    let result = std::thread::spawn(move || {
        compose(&worker_image, &snapshot, &AtomicBool::new(false)).unwrap()
    })
    .join()
    .unwrap();
    assert_eq!((result.width, result.height), (4, 4));
    let _apartment = Apartment::new().unwrap();
    let (png, _, _) = export::png_payload(&image, &editor).unwrap();
    let decoded = Image::from_encoded(&png).unwrap();
    assert_eq!(result.pixels, decoded.pixels);
    assert!(stamp.matches(&editor));
    assert!(source.same_storage(editor.layers().unwrap()));
    assert!(!result.pixels.same_storage(&image.pixels));
}

#[test]
fn cancellation_invalid_image_and_draft_reject_without_source_changes() {
    let image = image();
    let mut editor = Editor::default();
    assert_eq!(
        compose(&image, &editor, &AtomicBool::new(true))
            .err()
            .unwrap()
            .code(),
        E_ABORT
    );
    let invalid = Image {
        width: 0,
        ..image.clone()
    };
    assert_eq!(
        compose(&invalid, &editor, &AtomicBool::new(false))
            .err()
            .unwrap()
            .code(),
        E_INVALIDARG
    );
    editor.begin(Tool::Brush, Point::default()).unwrap();
    let stamp = editor.document_stamp();
    assert_eq!(
        compose(&image, &editor, &AtomicBool::new(false))
            .err()
            .unwrap()
            .code(),
        E_INVALIDARG
    );
    assert!(stamp.matches(&editor));
    assert!(editor.draft().is_some());
}

#[test]
fn composition_budget_is_retained_only_by_returned_pixels_and_released_on_failure() {
    let image = image();
    let mut editor = Editor::with_layers(Layers {
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let stamp = editor.document_stamp();
    let cancel = AtomicBool::new(false);
    let budget = crate::byte_budget::Budget::new(320);
    let result = compose_reserved(&image, &editor, &cancel, |bytes| budget.reserve(bytes)).unwrap();
    assert_eq!(budget.used(), 64);
    let clone = result.clone();
    drop(result);
    assert_eq!(budget.used(), 64);
    drop(clone);
    assert_eq!(budget.used(), 0);
    for limit in [63, 319] {
        let budget = crate::byte_budget::Budget::new(limit);
        assert_eq!(
            compose_reserved(&image, &editor, &cancel, |bytes| budget.reserve(bytes))
                .err()
                .unwrap()
                .code(),
            E_OUTOFMEMORY
        );
        assert_eq!(budget.used(), 0);
    }
    // 第三次预留是读回缓冲；拒绝它也必须释放已经合成的 WIC 位图。
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        compose_reserved(&image, &editor, &cancel, |bytes| {
            calls.set(calls.get() + 1);
            if calls.get() == 3 {
                None
            } else {
                budget.reserve(bytes)
            }
        })
        .err()
        .unwrap()
        .code(),
        E_OUTOFMEMORY
    );
    assert_eq!(budget.used(), 0);
    assert!(stamp.matches(&editor));
}
