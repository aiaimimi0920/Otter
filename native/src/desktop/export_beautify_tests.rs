//! 美化布局、真实像素、阴影方向和失败隔离；不读取屏幕或用户资源。
use super::*;
use serde_json::json;
fn settings(value: serde_json::Value) -> Settings {
    Settings::read(&json!({"beautify":value})).unwrap().unwrap()
}
#[test]
fn padding_white_card_and_premultiplied_content_match_compositing_order() {
    let inner = Image {
        width: 2,
        height: 1,
        pixels: vec![0, 0, 0, 0, 0, 0, 128, 128].into(),
    };
    let style = settings(json!({"enabled":true,"padding":2,"backgroundId":"graphite"}));
    let output = compose(&inner, &style, &AtomicBool::new(false)).unwrap();
    assert_eq!((output.width, output.height), (6, 5));
    assert_eq!(&output.pixels[..4], &[51, 41, 31, 255]);
    let first = (2 * 6 + 2) * 4;
    assert_eq!(&output.pixels[first..first + 4], &[255; 4]);
    assert_eq!(&output.pixels[first + 4..first + 8], &[127, 127, 255, 255]);
}
#[test]
fn rounded_corners_reveal_background_and_gradient_is_diagonal() {
    let inner = Image {
        width: 20,
        height: 10,
        pixels: [0, 0, 255, 255].repeat(200).into(),
    };
    let style =
        settings(json!({"enabled":true,"padding":2,"cornerRadius":100,"backgroundId":"white"}));
    let output = compose(&inner, &style, &AtomicBool::new(false)).unwrap();
    let corner = (2 * 24 + 2) * 4;
    assert_eq!(&output.pixels[corner..corner + 4], &[255; 4]);
    let center = (7 * 24 + 12) * 4;
    assert_eq!(&output.pixels[center..center + 4], &[0, 0, 255, 255]);
    let gradient = settings(json!({"enabled":true,"padding":4,"backgroundId":"unknown"}));
    assert_eq!(gradient.colors, ([255, 126, 95], [254, 180, 123]));
    let output = compose(&inner, &gradient, &AtomicBool::new(false)).unwrap();
    assert!(output.pixels[1] < output.pixels[output.pixels.len() - 3]);
}
#[test]
fn shadow_has_downward_offset_and_does_not_change_inner_content() {
    let inner = Image {
        width: 16,
        height: 16,
        pixels: [0, 0, 255, 255].repeat(256).into(),
    };
    let style = settings(json!({"enabled":true,"padding":12,"shadow":true,"backgroundId":"white"}));
    let output = compose(&inner, &style, &AtomicBool::new(false)).unwrap();
    let top = output.pixels[(8 * 40 + 20) * 4];
    let bottom = output.pixels[(31 * 40 + 20) * 4];
    assert!(bottom < top, "top={top} bottom={bottom}");
    assert_eq!(
        &output.pixels[(20 * 40 + 20) * 4..(20 * 40 + 20) * 4 + 4],
        &[0, 0, 255, 255]
    );
    let fractional = settings(json!({"enabled":true,"padding":16.6}));
    assert_eq!(fractional.padding, 17);
    assert_eq!(fractional.shadow_sigma, 4.0);
}
#[test]
fn disabled_cancelled_and_oversized_styles_do_not_produce_candidates() {
    assert!(Settings::read(&json!({})).unwrap().is_none());
    assert!(Settings::read(&json!({"beautify":{"enabled":true,"padding":9000}})).is_err());
    let inner = Image {
        width: 2,
        height: 2,
        pixels: vec![255; 16].into(),
    };
    let style = settings(json!({"enabled":true,"padding":8192}));
    assert!(compose(&inner, &style, &AtomicBool::new(false)).is_err());
    let style = settings(json!({"enabled":true}));
    assert!(compose(&inner, &style, &AtomicBool::new(true)).is_err());
    assert!(inner.pixels.iter().all(|byte| *byte == 255));
}
