//! 真实短命 child 与预乘像素；小尺寸内存夹具，不读取用户图片。
use super::*;
#[path = "surface_webp_browser_tests.rs"]
mod browser;
#[path = "surface_webp_color_tests.rs"]
mod color;
#[path = "surface_webp_exif_tests.rs"]
mod exif;
impl Image {
    pub(crate) fn webp_for_test() -> Vec<u8> {
        let mut bytes = Vec::new();
        image_webp::WebPEncoder::new(&mut bytes)
            .encode(
                &[255, 0, 0, 255, 0, 255, 0, 128],
                2,
                1,
                image_webp::ColorType::Rgba8,
            )
            .unwrap();
        bytes
    }
}
#[test]
fn surface_webp_child_lossless_alpha_and_wrong_mime() {
    let bytes = Image::webp_for_test();
    for mime in ["image/webp", "image/png"] {
        let image = Image::from_surface(&bytes, mime, None, || false).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(&*image.pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
    }
}
#[test]
fn surface_webp_child_rgb_expands_in_place() {
    let mut bytes = Vec::new();
    image_webp::WebPEncoder::new(&mut bytes)
        .encode(&[255, 0, 0, 0, 255, 0], 2, 1, image_webp::ColorType::Rgb8)
        .unwrap();
    let image = Image::from_surface(&bytes, "image/webp", None, || false).unwrap();
    assert_eq!(&*image.pixels, &[0, 0, 255, 255, 0, 255, 0, 255]);
}
#[test]
fn surface_webp_preflight_rejects_truncated_size_duplicate_and_animation() {
    use crate::surface_webp_header::inspect;
    let bytes = Image::webp_for_test();
    assert_eq!(inspect(&bytes).unwrap(), (2, 1));
    for size in 0..bytes.len() {
        assert!(inspect(&bytes[..size]).is_err());
    }
    let mut duplicate = bytes.clone();
    duplicate.extend_from_slice(&bytes[12..]);
    let size = (duplicate.len() - 8) as u32;
    duplicate[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(inspect(&duplicate).is_err());
    let mut animated = bytes.clone();
    animated.extend_from_slice(b"ANIM\x06\0\0\0\0\0\0\0\0\0");
    let size = (animated.len() - 8) as u32;
    animated[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(inspect(&animated).is_err());
    let mut mismatch = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0".to_vec();
    mismatch.extend_from_slice(&[0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let chunk = bytes.windows(4).position(|v| v == b"VP8L").unwrap();
    mismatch.extend_from_slice(&bytes[chunk..]);
    let size = (mismatch.len() - 8) as u32;
    mismatch[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(inspect(&mismatch).is_err());
}
#[test]
fn surface_webp_cancelled_before_or_during_pipe_never_returns_pixels() {
    let bytes = Image::webp_for_test();
    for stop in [1, 2, 3, 4] {
        let calls = std::cell::Cell::new(0);
        assert!(Image::from_surface(&bytes, "image/webp", None, || {
            calls.set(calls.get() + 1);
            calls.get() >= stop
        })
        .is_err());
    }
}
