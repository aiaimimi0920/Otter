//! 普通 Surface 的浏览器图片上下文兼容；不外推 Tile/Wall 的严格格式合同。
use super::*;
#[test]
fn surface_format_static_png_uses_bytes_not_supplied_mime() {
    let _com = apartment();
    for mime in [
        "IMAGE/PNG",
        "image/PNG; charset=binary",
        "image/jpeg",
        "image/gif",
        "image/webp",
        "image/unknown",
        "application/octet-stream",
        "text/plain",
        "application/xml",
        "image/x-test+xml",
        "image/png;broken=\"",
        "image/ png",
    ] {
        let image = Image::from_surface(&png(), mime, None, || false).unwrap();
        assert_eq!((image.width, image.height), (2, 1), "{mime}");
        assert_eq!(&*image.pixels, &[0, 0, 255, 255, 0, 128, 0, 128], "{mime}");
    }
}
#[test]
fn surface_format_actual_jpeg_keeps_exif_despite_png_declaration() {
    let _com = apartment();
    let image = Image::from_surface(&Image::oriented_jpeg_for_test(), "image/png", None, || {
        false
    })
    .unwrap();
    assert_eq!((image.width, image.height), (32, 48));
}
#[test]
fn surface_format_sniffing_never_accepts_script_or_unsupported_bytes() {
    let _com = apartment();
    for bytes in [
        b"<svg xmlns='http://www.w3.org/2000/svg'/>".as_slice(),
        b"<html><script>alert(1)</script>",
        b"RIFF\x14\0\0\0WEBPVP8 ",
        b"\x89PNG\r\n\x1a\n",
        b"GIF89a",
        b"BM",
        b"\xff\xd8\xff",
    ] {
        assert!(Image::from_surface(bytes, "image/png", None, || false).is_err());
    }
}
#[test]
fn surface_format_actual_animation_is_rejected_even_if_mime_is_jpeg() {
    let _com = apartment();
    let mut bytes = png();
    let mut chunk = vec![0, 0, 0, 8, b'a', b'c', b'T', b'L', 0, 0, 0, 2, 0, 0, 0, 0];
    chunk.extend_from_slice(&crc(&chunk[4..]).to_be_bytes());
    bytes.splice(33..33, chunk);
    let error = Image::from_surface(&bytes, "image/jpeg", None, || false)
        .err()
        .unwrap();
    assert!(error.message().contains("Animated Surface PNG"));
}
