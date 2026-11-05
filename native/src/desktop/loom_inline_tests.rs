//! 真实 WIC 合成/编解码链；夹具只在内存中，不访问用户剪贴板、文件或网络。
use super::*;
use crate::{crop_geometry::Rect, erase_layers::Layers};
fn decode_base64(text: &str) -> Vec<u8> {
    let mut result = Vec::new();
    let (mut bits, mut accumulator) = (0u32, 0u32);
    for byte in text.bytes().take_while(|byte| *byte != b'=') {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("Invalid Base64"),
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((accumulator >> bits) as u8);
            accumulator &= (1 << bits) - 1;
        }
    }
    result
}
#[test]
fn real_png_input_preserves_pixels_and_emits_decodable_base64() {
    let _apartment = Apartment::new().unwrap();
    let pixels: Pixels = vec![10, 20, 30, 255, 40, 50, 60, 255].into();
    let resource = prepare(
        2,
        1,
        pixels.clone(),
        &Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let Input::InlinePng {
        base64,
        width,
        height,
    } = resource.descriptor()
    else {
        panic!("inline expected");
    };
    assert_eq!((width, height), (2, 1));
    let png = decode_base64(base64);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let decoded = Image::from_encoded(&png).unwrap();
    assert_eq!((decoded.width, decoded.height), (width, height));
    assert_eq!(&decoded.pixels[..], &pixels[..]);
    println!("PNG_BASE64_FIXTURE:{base64}");
}
#[test]
fn committed_crop_and_annotation_texture_override_stale_host_image() {
    let _apartment = Apartment::new().unwrap();
    let base: Pixels = vec![0; 8 * 8 * 4].into();
    let mut editor = Editor::with_layers(Layers {
        size: (8, 8),
        base,
        annotations: Some(vec![0, 0, 255, 255, 0, 255, 0, 255].into()),
        annotation_size: Some((2, 1)),
    })
    .unwrap();
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 3.0,
            width: 4.0,
            height: 2.0,
        })
        .unwrap();
    let stamp = editor.document_stamp();
    let resource = prepare(
        1,
        1,
        vec![255, 0, 0, 255].into(),
        &editor,
        &AtomicBool::new(false),
    )
    .unwrap();
    let Input::InlinePng {
        base64,
        width,
        height,
    } = resource.descriptor()
    else {
        panic!("inline expected");
    };
    assert_eq!((width, height), (4, 2));
    let decoded = Image::from_encoded(&decode_base64(base64)).unwrap();
    assert_eq!(&decoded.pixels[..4], &[0, 0, 255, 255]);
    assert_eq!(&decoded.pixels[12..16], &[0, 255, 0, 255]);
    assert!(stamp.matches(&editor));
}
#[test]
fn completed_resource_does_not_retain_source_pixels_and_cancel_returns_no_candidate() {
    let (resource, source) = {
        let pixels: Pixels = vec![0, 0, 0, 255].into();
        let source = pixels.downgrade();
        let resource = prepare(1, 1, pixels, &Editor::default(), &AtomicBool::new(false)).unwrap();
        (resource, source)
    };
    assert!(source.upgrade().is_none());
    assert!(matches!(
        resource.descriptor(),
        Input::InlinePng {
            width: 1,
            height: 1,
            ..
        }
    ));
    let error = prepare(
        1,
        1,
        vec![0; 4].into(),
        &Editor::default(),
        &AtomicBool::new(true),
    )
    .err()
    .unwrap();
    assert_eq!(error.code(), E_ABORT);
}
