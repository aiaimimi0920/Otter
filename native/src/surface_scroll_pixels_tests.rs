//! WIC/Direct2D 实际绘制滚动后内容，验证裁剪没有随内容一起漂移。
use super::*;
#[test]
fn surface_scroll_pixels_shift_content_inside_stationary_clip() {
    let json = br#"{"id":"root","type":"scroll","layout":{"width":32,"height":32},"children":[{"id":"red","type":"view","layout":{"height":32,"minHeight":32},"style":{"background":"red"}},{"id":"blue","type":"view","layout":{"height":32,"minHeight":32},"style":{"background":"blue"}}]}"#;
    let before = render(json);
    let after = render_prepared(json, true, Default::default(), |p| {
        assert!(p.wheel([10., 10.], 1, 32., false).unwrap());
    });
    let pixel = |pixels: &[u8], x: usize, y: usize| {
        pixels[(y * WIDTH + x) * 4..(y * WIDTH + x + 1) * 4].to_vec()
    };
    assert_eq!(pixel(&before, 10, 10), [0, 0, 255, 255]);
    assert_eq!(pixel(&after, 10, 10), [255, 0, 0, 255]);
    assert_eq!(pixel(&after, 10, 40), [0, 0, 0, 0]);
    assert_eq!(pixel(&after, 40, 10), [0, 0, 0, 0]);
    // 中性色轨道与 thumb 真实落到 D2D 像素，滚动后 thumb 移到轨道末端。
    assert_eq!(pixel(&before, 27, 3), [159, 154, 146, 255]);
    assert_eq!(pixel(&after, 27, 3), [32, 23, 17, 255]);
    assert_eq!(pixel(&after, 27, 29), [159, 154, 146, 255]);
}
