//! 准备阶段只读输入，使用仓库夹具，不创建文件或显示对话框。
use super::*;
#[path = "open_capture_tests.rs"]
mod capture;

#[test]
fn prepare_decodes_image_and_pin_region_before_replacement() {
    let _apartment = super::super::Apartment::new().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let prepared =
        PreparedDocument::open(&root.join("native/assets/branding/32x32.png"), true).unwrap();
    assert_eq!((prepared.image.width, prepared.image.height), (32, 32));
    assert_eq!(prepared.image.pixels.len(), 32 * 32 * 4);
}

#[test]
fn invalid_image_fails_without_replacing_existing_canvas() {
    let _apartment = super::super::Apartment::new().unwrap();
    let original = Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    };
    let mut canvas = super::super::canvas::Canvas::new(Some(original.clone())).unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    if let Ok(prepared) = PreparedDocument::open(&path, false) {
        canvas.replace_image(prepared.image).unwrap();
        panic!("non-image unexpectedly decoded");
    }
    assert!(canvas
        .image
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&original.pixels));
}

#[test]
fn image_preparation_stops_during_pin_mask_scan_and_releases_pixels() {
    for stop_at in [1, 4] {
        let image = Image {
            width: 64,
            height: 256,
            pixels: vec![255; 64 * 256 * 4].into(),
        };
        let weak = image.pixels.downgrade();
        let checks = std::cell::Cell::new(0);
        let result = PreparedDocument::from_image_with_cancel(image, true, || {
            checks.set(checks.get() + 1);
            checks.get() >= stop_at
        });
        assert_eq!(result.err().unwrap().code(), E_ABORT);
        assert!(
            checks.get() < 16,
            "cancelled preparation must stop before scanning every row"
        );
        assert!(weak.upgrade().is_none());
    }
}
