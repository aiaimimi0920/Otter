//! 准备阶段只读输入，使用仓库夹具，不创建文件或显示对话框。
use super::*;
#[path = "open_capture_tests.rs"]
mod capture;

#[test]
fn prepare_decodes_image_and_pin_region_before_replacement() {
    let _apartment = super::super::Apartment::new().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let prepared = PreparedDocument::open(&root.join("src-tauri/icons/32x32.png"), true).unwrap();
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
