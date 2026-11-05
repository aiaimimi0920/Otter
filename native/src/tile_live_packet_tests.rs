//! 原协议golden和独立像素常量，覆盖raw透明及WIC PNG；不以新实现生成expected。
use super::*;
fn golden(name: &str) -> Vec<u8> {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/wall/wall-media.v1.json")).unwrap();
    let hex = value[name].as_str().unwrap();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn raw_straight_alpha_is_premultiplied_once_and_cancel_or_wrong_codec_does_not_decode() {
    let mut bytes = golden("raw_bgra");
    bytes[80..84].copy_from_slice(&[100, 50, 200, 128]);
    let f = decode(&bytes, WallMediaFormat::RawBgra, (0, 0), 0, || false).unwrap();
    assert_eq!(&f.image.pixels[..4], &[50, 25, 100, 128]);
    assert_eq!(f.identity, (2, 7));
    assert!(decode(&bytes, WallMediaFormat::Png, (0, 0), 0, || false).is_err());
    assert!(decode(&bytes, WallMediaFormat::RawBgra, (2, 7), 0, || false).is_err());
    assert!(decode(&bytes, WallMediaFormat::RawBgra, (0, 0), 0, || true).is_err());
    let expected = u64::from_be_bytes(bytes[64..72].try_into().unwrap());
    assert!(decode(
        &bytes,
        WallMediaFormat::RawBgra,
        (0, 0),
        expected + 1,
        || false
    )
    .is_err());
}
#[test]
fn png_golden_uses_actual_wic_dimensions_and_bgra_pixels() {
    use windows::Win32::System::Com::*;
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
    }
    let bytes = golden("png");
    let f = decode(&bytes, WallMediaFormat::Png, (0, 0), 0, || false).unwrap();
    assert_eq!((f.image.width, f.image.height), (2, 1));
    assert_eq!(&*f.image.pixels, &[0, 0, 255, 255, 255, 0, 0, 255]);
    unsafe {
        CoUninitialize();
    }
}
