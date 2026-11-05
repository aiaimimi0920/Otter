//! 生产 Presentation 的无像素替代内容：真实 D2D 裁剪、文字与成功替换。
use super::*;
fn style() -> crate::surface_text::Style<'static> {
    crate::surface_text::Style {
        family: "Segoe UI",
        size: 16.,
        weight: 400,
        whitespace: crate::surface_text::Whitespace::Normal,
        align: crate::surface_text::Align::Start,
        line_height: None,
    }
}
fn scene(alt: &str, width: u32, opacity: f32, radius: u32) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"root","type":"column","children":[{"id":"image","type":"image","props":{"src":"data:image/png,bad","alt":alt},"layout":{"width":width,"height":32},"style":{"color":"#f7f8ef","opacity":opacity,"borderRadius":radius}}]})).unwrap()).unwrap()
}
fn draw(frame: &crate::surface_presentation::Presentation) -> Vec<u8> {
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let (bitmap, target) = target(&factory);
        let brush = target
            .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
            .unwrap();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        frame.draw(&target, &brush).unwrap();
        target.EndDraw(None, None).unwrap();
        pixels(&bitmap)
    }
}
fn render(alt: &str, width: u32, opacity: f32, radius: u32) -> Vec<u8> {
    let frame = crate::surface_presentation::Presentation::new(
        scene(alt, width, opacity, radius).with_image_availability(|_| false),
        style(),
        context(),
        [64., 64.],
    )
    .unwrap();
    for text in frame.text().items() {
        assert!(!text.selectable());
    }
    draw(&frame)
}
#[test]
fn inline_image_placeholder_paints_alt_icon_and_clips_without_pixels() {
    let _com = apartment();
    let empty = render("", 64, 1., 0);
    let named = render("缺图", 64, 1., 0);
    assert!((0..16).any(|y| (0..16).any(|x| pixel(&empty, x, y)[3] > 0)));
    assert!((0..32).all(|y| (20..64).all(|x| pixel(&empty, x, y)[3] == 0)));
    assert!((0..32).any(|y| (20..64).any(|x| pixel(&named, x, y)[3] > 0)));
    assert!((32..64).all(|y| (0..64).all(|x| pixel(&named, x, y)[3] == 0)));
    let narrow = render("very long alternative", 8, 1., 0);
    assert!((0..64).all(|y| (8..64).all(|x| pixel(&narrow, x, y)[3] == 0)));
    let rounded = render("缺图", 64, 1., 12);
    assert_eq!(pixel(&rounded, 0, 0)[3], 0);
    let faded = render("缺图", 64, 0.5, 0);
    assert!(faded.chunks_exact(4).all(|p| p[3] <= 128));
    if let Some(path) = std::env::var_os("OTTER_IMAGE_PLACEHOLDER_PNG") {
        // 显式测试产物路径由 runner 放入 linshi；仅导出本测试的合成像素。
        let mut rgba = named;
        for p in rgba.chunks_exact_mut(4) {
            p.swap(0, 2);
            if p[3] != 0 {
                for i in 0..3 {
                    p[i] = (u16::from(p[i]) * 255 / u16::from(p[3])).min(255) as u8;
                }
            }
        }
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, 64, 64);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&rgba)
            .unwrap();
    }
}
#[test]
fn inline_image_placeholder_success_replacement_removes_alt_paint() {
    let _com = apartment();
    let mut frame = crate::surface_presentation::Presentation::new(
        scene("缺图", 64, 1., 0).with_image_availability(|_| false),
        style(),
        context(),
        [64., 64.],
    )
    .unwrap();
    assert_eq!(frame.text().items().len(), 1);
    let before = draw(&frame);
    frame
        .replace_with_images(
            scene("缺图", 64, 1., 0).with_image_availability(|_| true),
            style(),
            context(),
            [64., 64.],
            [("image".to_owned(), source())].into(),
        )
        .unwrap();
    assert!(frame.text().items().is_empty());
    let after = draw(&frame);
    assert_ne!(before, after);
    assert_eq!(pixel(&after, 8, 8), [0, 0, 255, 255]);
    assert_eq!(pixel(&after, 48, 8), [0, 255, 0, 255]);
}
