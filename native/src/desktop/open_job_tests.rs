//! Background-task/revision-guard tests show no file dialogs and touch no real user data.
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
    // Pin hit regions also share original pixels; release their owner before verifying cancelled tasks retain no references.
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
fn timer_failure_never_admits_a_decoder_or_rolls_back_an_unreserved_timer() {
    let bytes: crate::pixels::Pixels = vec![0; 4].into();
    let weak = bytes.downgrade();
    let result = start_worker(
        || false,
        || panic!("no timer to roll back"),
        move |_| {
            drop(bytes);
            panic!("decoder must not start");
        },
    );
    assert_eq!(result.err().unwrap().code(), E_FAIL);
    assert!(weak.upgrade().is_none());
}

#[test]
fn spawn_failure_rolls_back_exactly_the_reserved_timer() {
    let steps = std::cell::Cell::new(0);
    let advance = |expected| assert_eq!(steps.replace(expected + 1), expected);
    let result = start_worker(
        || {
            advance(0);
            true
        },
        || advance(2),
        |_| {
            advance(1);
            Err(std::io::Error::other("fixture spawn failure"))
        },
    );
    assert_eq!(result.err().unwrap().message(), "fixture spawn failure");
    assert_eq!(steps.get(), 3);
}

#[test]
fn worker_initializes_own_com_and_returns_cpu_owned_image() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("native/assets/branding/32x32.png");
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
fn file_and_encoded_decoders_observe_stop_before_returning_pixels() {
    use super::super::image::Image;
    let _apartment = Apartment::new().unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/branding/32x32.png");
    let png = std::fs::read(&path).unwrap();
    for stop_on in [1, 2] {
        for encoded in [false, true] {
            let checks = std::cell::Cell::new(0);
            let stopped = || {
                checks.set(checks.get() + 1);
                checks.get() >= stop_on
            };
            let result = if encoded {
                Image::from_encoded_with_cancel(&png, &stopped)
            } else {
                Image::open_with_cancel(&path, &stopped)
            };
            assert_eq!(result.err().unwrap().message(), "Image decode cancelled");
            assert_eq!(checks.get(), stop_on);
        }
    }
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
