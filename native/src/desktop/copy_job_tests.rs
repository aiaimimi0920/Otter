//! 只验证后台编码及发布准入，不读写系统剪贴板、不生成磁盘文件。
use super::*;
use crate::{desktop::image::Image, editor::Tool, viewport::Point};

fn canvas() -> Canvas {
    Canvas::new(Some(Image {
        width: 8,
        height: 8,
        pixels: [0, 0, 64, 128].repeat(64).into(),
    }))
    .unwrap()
}
fn finish(job: &mut Job) -> Output {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !job.worker.as_ref().unwrap().is_finished() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    job.worker.take().unwrap().join().unwrap()
}
fn decode(png: &[u8]) -> (u32, u32, Vec<u8>) {
    use windows::Win32::{Graphics::Imaging::*, System::Com::*};
    let _apartment = super::super::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = wic.CreateStream().unwrap();
        stream.InitializeFromMemory(png).unwrap();
        let decoder = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
            .unwrap();
        let frame = decoder.GetFrame(0).unwrap();
        let (mut width, mut height) = (0, 0);
        frame.GetSize(&mut width, &mut height).unwrap();
        let converter = wic.CreateFormatConverter().unwrap();
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .unwrap();
        let mut pixels = vec![0; (width * height * 4) as usize];
        converter
            .CopyPixels(std::ptr::null(), width * 4, &mut pixels)
            .unwrap();
        (width, height, pixels)
    }
}

#[test]
fn displayed_image_is_copied_and_replaced_display_cannot_publish() {
    let canvas = Canvas::new(None).unwrap();
    let image = Image {
        width: 1,
        height: 1,
        pixels: vec![31, 41, 51, 255].into(),
    };
    let source = crate::desktop::document_state::new_unit_id().unwrap();
    let frame = crate::desktop::display_frame::Frame::capture(source.clone(), image.clone(), None);
    let mut job = prepare(&canvas, Some(&frame)).unwrap().unwrap();
    let ready = finish(&mut job).unwrap().unwrap();
    let editable = ready.editable.unwrap();
    assert!(editable.capture_source.is_none());
    let pasted = State::from_prepared(editable.prepare(|| false).unwrap()).unwrap();
    assert_eq!(
        pasted
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .as_ref(),
        &[31, 41, 51, 255]
    );
    assert!(pasted.canvas.borrow().editor.elements().is_empty());
    assert!(pasted.resolved_display.borrow().is_none());
    assert!(pasted.live_request.borrow().is_none());
    let payload = ready.payload;
    assert_eq!(decode(&payload.png), (1, 1, vec![31, 41, 51, 255]));
    assert!(eligible(&canvas, Some(&frame), &job));
    assert!(!eligible(&canvas, None, &job));
    let replacement = crate::desktop::display_frame::Frame::capture(source, image, None);
    assert!(!eligible(&canvas, Some(&replacement), &job));
    assert!(canvas.image.is_none());
}

#[test]
fn displayed_annotations_and_crop_are_exported_instead_of_target_document() {
    let source = canvas();
    let mut editor = source.editor.export_snapshot();
    editor.begin(Tool::Brush, Point { x: 3.5, y: 3.5 }).unwrap();
    assert!(editor.commit());
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let frame = crate::desktop::display_frame::Frame::capture(
        crate::desktop::document_state::new_unit_id().unwrap(),
        source.image.clone().unwrap(),
        Some(&editor),
    );
    let mut target = canvas();
    target.attach_capture(Some(
        crate::desktop::canvas::CaptureSource::from_session(
            r#"{"kind":"region","futureClipboard":{"wrongTarget":true}}"#,
        )
        .unwrap(),
    ));
    let mut job = prepare(&target, Some(&frame)).unwrap().unwrap();
    let ready = finish(&mut job).unwrap().unwrap();
    let content = ready.editable.unwrap();
    // The displayed editor is retained, but the target's unrelated capture provenance is not.
    assert!(content.capture_source.is_none());
    let first = State::from_prepared(content.prepare(|| false).unwrap()).unwrap();
    let second = State::from_prepared(content.prepare(|| false).unwrap()).unwrap();
    let first = first.canvas.borrow();
    let second = second.canvas.borrow();
    assert_eq!(first.editor.annotation_frame(), editor.annotation_frame());
    assert_eq!(first.editor.elements().len(), 1);
    assert!(!editor.document_stamp().same_document(&first.editor));
    assert!(!first.editor.document_stamp().same_document(&second.editor));
    assert_ne!(
        first.editor.elements()[0].annotation_id(),
        second.editor.elements()[0].annotation_id()
    );
    assert!(first.capture_source().is_none());
    let payload = ready.payload;
    let (width, height, pixels) = decode(&payload.png);
    assert_eq!((width, height), (4, 4));
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != [0, 0, 64, 128]));
    assert!(eligible(&target, Some(&frame), &job));
}

#[test]
fn worker_preserves_crop_annotations_and_transparency_without_clipboard_access() {
    let mut canvas = canvas();
    let style = canvas
        .editor
        .style()
        .with_color(crate::stroke_style::Color(255, 0, 0))
        .with_width(2.0)
        .unwrap();
    canvas.editor.set_style(style);
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 3.5, y: 3.5 })
        .unwrap();
    assert!(canvas.editor.commit());
    canvas
        .editor
        .crop_to(crate::crop_geometry::Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let mut job = prepare(&canvas, None).unwrap().unwrap();
    let png = finish(&mut job).unwrap().unwrap().payload;
    assert!(eligible(&canvas, None, &job));
    let (width, height, pixels) = decode(&png.png);
    assert_eq!(i32::from_le_bytes(png.dibv5[4..8].try_into().unwrap()), 4);
    assert_eq!(i32::from_le_bytes(png.dibv5[8..12].try_into().unwrap()), -4);
    assert_eq!(png.dibv5.len(), 124 + pixels.len());
    for (dib, expected) in png.dibv5[124..].chunks_exact(4).zip(pixels.chunks_exact(4)) {
        assert_eq!(dib[3], expected[3]);
        for channel in 0..3 {
            let premultiplied = (dib[channel] as i32 * dib[3] as i32 + 127) / 255;
            assert!((premultiplied - expected[channel] as i32).abs() <= 1);
        }
    }
    assert_eq!((width, height), (4, 4));
    assert!(pixels.chunks_exact(4).any(|p| p == [0, 0, 255, 255]));
    assert!(pixels.chunks_exact(4).any(|p| p[3] == 128));
}

#[test]
fn completed_copy_cannot_publish_after_cancel_or_document_replacement() {
    let mut canvas = canvas();
    let mut job = prepare(&canvas, None).unwrap().unwrap();
    let png = finish(&mut job).unwrap().unwrap().payload;
    assert!(eligible(&canvas, None, &job));
    job.cancel.store(true, Ordering::Release);
    assert!(!eligible(&canvas, None, &job));
    job.cancel.store(false, Ordering::Release);
    canvas.editor = canvas.editor.export_snapshot();
    assert!(!eligible(&canvas, None, &job));
    let weak = png.png.downgrade();
    let weak_dib = png.dibv5.downgrade();
    drop(png);
    assert!(weak.upgrade().is_none());
    assert!(weak_dib.upgrade().is_none());
}

#[test]
fn active_gesture_rejects_result_and_drop_joins_worker() {
    let mut canvas = canvas();
    let job = prepare(&canvas, None).unwrap().unwrap();
    canvas
        .editor
        .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
        .unwrap();
    assert!(!eligible(&canvas, None, &job));
    let stamp = canvas.editor.document_stamp();
    drop(job);
    assert!(stamp.matches(&canvas.editor));
    assert!(prepare(&Canvas::new(None).unwrap(), None)
        .unwrap()
        .is_none());
}
