//! 后台任务和版本防护测试不显示文件对话框，也不触及真实用户数据。
use super::*;
use crate::{editor::Tool, viewport::Point};

#[test]
fn captured_memory_image_is_prepared_without_copy_and_released_on_cancel() {
    let image = super::super::image::Image {
        width: 2,
        height: 2,
        pixels: vec![255; 16].into(),
    };
    let pixels = image.pixels.clone();
    let task =
        std::thread::spawn(move || load_input(Input::Image(image), true, &AtomicBool::new(false)));
    let prepared = task.join().unwrap().unwrap().unwrap();
    assert!(prepared.image.pixels.same_storage(&pixels));
    assert_eq!((prepared.image.width, prepared.image.height), (2, 2));
    drop(pixels);
    // 钉图命中区域也共享原像素；其 owner 释放后才能验证取消任务不残留引用。
    drop(prepared.region);
    let weak = prepared.image.pixels.downgrade();
    assert!(
        load_input(Input::Image(prepared.image), true, &AtomicBool::new(true))
            .unwrap()
            .is_none()
    );
    assert!(weak.upgrade().is_none());
}

#[test]
fn scheduler_failure_cancels_and_joins_running_worker_without_retaining_input() {
    let canvas = Canvas::new(None).unwrap();
    let token = DocumentToken::capture(&canvas);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let bytes: crate::pixels::Pixels = vec![0; 4].into();
    let weak = bytes.downgrade();
    let worker = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !worker_cancel.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "worker was not cancelled"
            );
            std::thread::yield_now();
        }
        drop(bytes);
        Ok(None)
    });
    let slot = std::cell::RefCell::new(Some(OpenJob {
        new_pin: false,
        worker: Some(worker),
        cancel: cancel.clone(),
        token,
    }));
    assert!(scheduled(&slot, true).is_ok());
    assert!(slot.borrow().is_some());
    assert!(!cancel.load(Ordering::Acquire));
    assert!(scheduled(&slot, false).is_err());
    assert!(slot.borrow().is_none());
    assert!(cancel.load(Ordering::Acquire));
    assert!(weak.upgrade().is_none());
    assert!(canvas.image.is_none());
}

#[test]
fn scheduler_failure_discards_already_completed_document() {
    let canvas = Canvas::new(None).unwrap();
    let image = super::super::image::Image {
        width: 1,
        height: 1,
        pixels: vec![0; 4].into(),
    };
    let weak = image.pixels.downgrade();
    let worker = std::thread::spawn(move || {
        PreparedDocument::from_image(image, false)
            .map(|document| Some(vec![document]))
            .map_err(|error| error.to_string())
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !worker.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "worker did not complete"
        );
        std::thread::yield_now();
    }
    let slot = std::cell::RefCell::new(Some(OpenJob {
        new_pin: false,
        worker: Some(worker),
        cancel: Arc::new(AtomicBool::new(false)),
        token: DocumentToken::capture(&canvas),
    }));
    assert!(weak.upgrade().is_some());
    assert!(scheduled(&slot, false).is_err());
    assert!(weak.upgrade().is_none());
    assert!(slot.borrow().is_none());
    assert!(canvas.image.is_none());
}

#[test]
fn worker_initializes_own_com_and_returns_cpu_owned_image() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("src-tauri/icons/32x32.png");
    let task = std::thread::spawn(move || load(path, true, &AtomicBool::new(false)));
    let prepared = task.join().unwrap().unwrap().unwrap();
    assert_eq!((prepared.image.width, prepared.image.height), (32, 32));
    assert_eq!(prepared.image.pixels.len(), 4096);
}

#[test]
fn cancelled_request_does_not_attempt_to_open_invalid_path() {
    assert!(load(
        PathBuf::from("nonexistent-native-test-image"),
        true,
        &AtomicBool::new(true)
    )
    .unwrap()
    .is_none());
}

#[test]
fn encoded_clipboard_source_decodes_on_worker_and_releases_input_bytes() {
    let image = super::super::image::Image {
        width: 4,
        height: 4,
        pixels: [16, 32, 64, 128].repeat(16).into(),
    };
    let mut editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
        size: (4, 4),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 2.0,
            height: 2.0,
        })
        .unwrap();
    let payload = super::super::export::clipboard_payload(&image, &editor).unwrap();
    let weak = payload.png.downgrade();
    let result = std::thread::spawn(move || {
        load_input(Input::Encoded(payload.png), true, &AtomicBool::new(false))
    })
    .join()
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!((result.image.width, result.image.height), (2, 2));
    assert_eq!(result.image.pixels.len(), 16);
    for pixel in result.image.pixels.chunks_exact(4) {
        for (actual, expected) in pixel.iter().zip([16i32, 32, 64, 128]) {
            assert!((*actual as i32 - expected).abs() <= 1);
        }
    }
    assert!(weak.upgrade().is_none());
}

#[test]
fn invalid_or_cancelled_encoded_source_does_not_produce_document() {
    assert!(load_input(
        Input::Encoded(vec![0; 32].into()),
        false,
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(load_input(
        Input::Encoded(vec![0; 32].into()),
        false,
        &AtomicBool::new(true)
    )
    .unwrap()
    .is_none());
    let _apartment = super::super::Apartment::new().unwrap();
    assert!(super::super::image::Image::from_encoded(&[]).is_err());
}

#[test]
fn native_dibv5_copy_round_trips_through_background_paste_without_clipboard_access() {
    let image = super::super::image::Image {
        width: 2,
        height: 1,
        pixels: vec![10, 20, 30, 128, 0, 0, 0, 0].into(),
    };
    let editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
        size: (2, 1),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    let payload = super::super::export::clipboard_payload(&image, &editor).unwrap();
    let weak = payload.dibv5.downgrade();
    let result = std::thread::spawn(move || {
        load_input(Input::Dib(payload.dibv5), true, &AtomicBool::new(false))
    })
    .join()
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!((result.image.width, result.image.height), (2, 1));
    for (actual, expected) in result.image.pixels.iter().zip(image.pixels.iter()) {
        assert!((*actual as i32 - *expected as i32).abs() <= 1);
    }
    assert!(weak.upgrade().is_none());
    assert!(load_input(
        Input::Dib(vec![0; 32].into()),
        false,
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(load_input(
        Input::Dib(vec![0; 32].into()),
        false,
        &AtomicBool::new(true)
    )
    .unwrap()
    .is_none());
}

#[test]
fn changed_document_or_active_gesture_invalidates_pending_result() {
    let mut canvas = Canvas::new(None).unwrap();
    let token = DocumentToken::capture(&canvas);
    assert!(token.matches(&canvas));
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    assert!(!token.matches(&canvas));
    canvas.editor.cancel();
    assert!(token.matches(&canvas));
    canvas.selection_drag = Some((Point::default(), Point::default()));
    assert!(!token.matches(&canvas));
    canvas.selection_drag = None;
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    canvas.editor.commit();
    canvas.editor.undo();
    assert!(
        !token.matches(&canvas),
        "undo to visually empty is still a different revision"
    );
}

#[test]
fn source_identity_prevents_revision_reset_from_accepting_stale_result() {
    let image = super::super::image::Image {
        width: 1,
        height: 1,
        pixels: vec![0; 4].into(),
    };
    let mut canvas = Canvas::new(Some(image.clone())).unwrap();
    let token = DocumentToken::capture(&canvas);
    canvas
        .replace_image(super::super::image::Image {
            width: 1,
            height: 1,
            pixels: vec![0; 4].into(),
        })
        .unwrap();
    assert!(!token.matches(&canvas));
    assert_eq!(canvas.editor.revision(), 0);
}

#[test]
fn replacement_with_shared_pixels_is_still_a_different_document() {
    let image = super::super::image::Image {
        width: 1,
        height: 1,
        pixels: vec![0; 4].into(),
    };
    let mut canvas = Canvas::new(Some(image.clone())).unwrap();
    let token = DocumentToken::capture(&canvas);
    canvas.replace_image(image).unwrap();
    assert_eq!(canvas.editor.revision(), 0);
    assert!(!token.matches(&canvas));
}

#[test]
fn replacing_empty_editor_or_cloning_document_invalidates_pending_result() {
    let mut canvas = Canvas::new(None).unwrap();
    let token = DocumentToken::capture(&canvas);
    canvas.editor = crate::editor::Editor::default();
    assert!(!token.matches(&canvas));
    let token = DocumentToken::capture(&canvas);
    canvas.editor = canvas.editor.export_snapshot();
    assert!(!token.matches(&canvas));
}
