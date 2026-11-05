//! 捕获交付不解码原图、不访问文件；验证来源与预览随新钉图/文档替换移交。
use super::*;
use crate::{capture_color::SdrFrame, capture_dynamic::Reason, capture_job::Captured};
struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let cache = self.0.join("clipboard_cache");
        if cache.exists() {
            for file in std::fs::read_dir(&cache).unwrap() {
                std::fs::remove_file(file.unwrap().path()).unwrap();
            }
            std::fs::remove_dir(cache).unwrap();
        }
        if self.0.exists() {
            std::fs::remove_dir(&self.0).unwrap();
        }
    }
}
#[test]
fn document_owns_original_lease_until_image_replacement() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let directory = Directory(
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .join(format!("otter-source-lease-{:?}", unsafe {
                windows::Win32::System::Com::CoCreateGuid().unwrap()
            })),
    );
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let bytes: Vec<_> = [0x4000u16; 4]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let raw = crate::capture_color::ScRgb::new(&bytes, 1, 1, 8).unwrap();
    let outcome = crate::capture_dynamic::Outcome {
        frame: crate::capture_dynamic::Frame::Hdr {
            image: raw.to_hdr(0.01, 1000.0, 1.0, &cancel).unwrap(),
            preview: raw.to_sdr(320.0, 1.0, &cancel).unwrap(),
        },
        reason: None,
        downgraded: false,
        display: None,
        diagnostic: None,
    };
    let captured = crate::capture_job::PreparedCapture::prepare(
        outcome,
        &directory.0,
        "source",
        Default::default(),
        &cancel,
    )
    .unwrap()
    .commit(&cancel, || true)
    .unwrap();
    let lease = std::sync::Arc::downgrade(captured.original_lease.as_ref().unwrap());
    let path = captured.original.clone().unwrap();
    let prepared = PreparedDocument::from_capture(captured, true, None).unwrap();
    let state = State::from_prepared(prepared).unwrap();
    assert!(lease.upgrade().is_some());
    assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
    state
        .canvas
        .borrow_mut()
        .replace_image(Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        })
        .unwrap();
    assert!(lease.upgrade().is_none());
    assert!(std::fs::OpenOptions::new().write(true).open(&path).is_ok());
    assert!(state.canvas.borrow().capture_source().is_none());
}
fn captured() -> Captured {
    Captured {
        backend: "wgc",
        original_lease: None,
        preview: SdrFrame {
            width: 1,
            height: 1,
            pixels: vec![188, 188, 188, 255].into(),
        },
        original: Some(std::path::PathBuf::from("C:/unopened-hdr-source.png")),
        reason: Some(Reason::HdrFailed),
        downgraded: true,
        display: None,
        diagnostic: Some("fixture diagnostic".into()),
    }
}
#[test]
fn prepared_capture_keeps_preview_storage_source_and_pin_origin_without_reading_path() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let input = captured();
    let pixels = input.preview.pixels.clone();
    let prepared =
        PreparedDocument::from_capture(input, true, Some(POINT { x: -50, y: 20 })).unwrap();
    assert!(prepared.image.pixels.same_storage(&pixels));
    assert_eq!(prepared.capture_origin.unwrap().x, -50);
    let state = State::from_prepared(prepared).unwrap();
    let canvas = state.canvas.borrow();
    let source = canvas.capture_source().unwrap();
    assert!(source.downgraded);
    assert_eq!(source.reason, Some(Reason::HdrFailed));
    assert_eq!(source.diagnostic.as_deref(), Some("fixture diagnostic"));
    assert_eq!(
        source.original.as_deref(),
        Some(Path::new("C:/unopened-hdr-source.png"))
    );
    assert!(canvas.image.as_ref().unwrap().pixels.same_storage(&pixels));
}
#[test]
fn replacing_or_changing_document_identity_does_not_expose_previous_capture_source() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let prepared = PreparedDocument::from_capture(captured(), false, None).unwrap();
    let mut canvas = super::super::super::canvas::Canvas::new(None).unwrap();
    canvas.replace_image(prepared.image.clone()).unwrap();
    canvas.attach_capture(prepared.capture_source);
    assert!(canvas.capture_source().is_some());
    canvas.editor = crate::editor::Editor::default();
    canvas.sync_raster_source();
    assert!(canvas.capture_source().is_none());
    let prepared = PreparedDocument::from_capture(captured(), false, None).unwrap();
    canvas.attach_capture(prepared.capture_source);
    assert!(canvas.capture_source().is_some());
    canvas.replace_image(prepared.image).unwrap();
    assert!(canvas.capture_source().is_none());
}
