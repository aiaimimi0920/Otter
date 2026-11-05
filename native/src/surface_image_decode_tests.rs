//! 真实 WIC 像素、尺寸准入与 RGBA 预乘；仅内存夹具，不读取用户图片。
use super::*;
#[path = "surface_image_format_tests.rs"]
mod format;
#[path = "surface_image_orientation_tests.rs"]
mod orientation;
#[path = "surface_svg_tests.rs"]
mod svg;
#[path = "surface_webp_tests.rs"]
mod webp;
struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
fn apartment() -> Com {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    Com
}
fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
        writer.finish().unwrap();
    }
    bytes
}
fn crc(bytes: &[u8]) -> u32 {
    let mut value = !0u32;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(value & 1));
        }
    }
    !value
}
#[test]
fn surface_decode_png_uses_natural_dimensions_and_shared_pixels() {
    let _com = apartment();
    let image = Image::from_surface(&png(), "image/png", Some((99, 99)), || false).unwrap();
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(&*image.pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
    let weak = image.pixels.downgrade();
    let clone = image.clone();
    assert!(clone.pixels.same_storage(&image.pixels));
    drop(image);
    assert!(weak.upgrade().is_some());
    drop(clone);
    assert!(weak.upgrade().is_none());
}
#[test]
fn surface_decode_rgba_premultiplies_and_cancels_between_rows() {
    let image = Image::from_surface(
        &[255, 100, 50, 128, 255, 255, 255, 0],
        "application/x-neuro-rgba8",
        Some((2, 1)),
        || false,
    )
    .unwrap();
    assert_eq!(&*image.pixels, &[25, 50, 128, 128, 0, 0, 0, 0]);
    let calls = std::cell::Cell::new(0);
    assert!(
        Image::from_surface(&[255; 8], "application/x-neuro-rgba8", Some((1, 2)), || {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        })
        .is_err()
    );
    assert!(
        Image::from_surface(&[0; 4], "application/x-neuro-rgba8", Some((2, 1)), || false).is_err()
    );
    assert!(Image::from_surface(
        &vec![0; 16385 * 4],
        "application/x-neuro-rgba8",
        Some((16385, 1)),
        || false
    )
    .is_err());
}
#[test]
fn surface_decode_rejects_mime_animation_corruption_and_pixel_bomb_before_copy() {
    let _com = apartment();
    let bytes = png();
    for mime in ["image/svg+xml", "IMAGE/SVG+XML; charset=utf-8"] {
        assert!(Image::from_surface(&bytes, mime, None, || false).is_err());
    }
    assert!(Image::from_surface(&bytes, "image/png", None, || true).is_err());
    assert!(Image::from_surface(&bytes[..20], "image/png", None, || false).is_err());
    let mut bomb = bytes.clone();
    bomb[16..20].copy_from_slice(&5000u32.to_be_bytes());
    bomb[20..24].copy_from_slice(&5000u32.to_be_bytes());
    let checksum = crc(&bomb[12..29]);
    bomb[29..33].copy_from_slice(&checksum.to_be_bytes());
    let Err(error) = Image::from_surface(&bomb, "image/png", None, || false) else {
        panic!("pixel bomb decoded")
    };
    assert!(error.message().contains("native decode budget"), "{error}");
    let mut animated = bytes;
    let mut chunk = vec![0, 0, 0, 8, b'a', b'c', b'T', b'L', 0, 0, 0, 2, 0, 0, 0, 0];
    chunk.extend_from_slice(&crc(&chunk[4..]).to_be_bytes());
    animated.splice(33..33, chunk);
    let Err(error) = Image::from_surface(&animated, "image/png", None, || false) else {
        panic!("animation silently flattened")
    };
    assert!(error.message().contains("Animated Surface PNG"));
}
