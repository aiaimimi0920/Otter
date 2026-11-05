//! 使用真实 WIC/D2D 验证预计算模糊标注，不读取用户数据。
use super::*;
#[test]
fn mapped_blur_texture_draws_at_logical_frame_not_pixel_dimensions() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 32,
        height: 24,
        pixels: [0, 0, 0, 255].repeat(32 * 24).into(),
    };
    let texture = (0..8 * 4)
        .flat_map(|i| {
            if i % 8 < 4 {
                [0, 0, 255, 255]
            } else {
                [255, 0, 0, 255]
            }
        })
        .collect::<Vec<_>>();
    let image = crate::blur_image::BlurImage::new(8, 4, texture.into(), 2)
        .unwrap()
        .with_frame(crate::crop_geometry::Rect {
            x: 8.0,
            y: 8.0,
            width: 16.0,
            height: 8.0,
        })
        .unwrap();
    let mut editor = Editor::default();
    editor.set_style(editor.style().with_width(8.0).unwrap());
    editor.begin_blur(Point { x: 8.0, y: 12.0 }, image).unwrap();
    editor.extend(Point { x: 24.0, y: 12.0 }).unwrap();
    assert!(editor.commit());
    let output = tests::decode_png(encode(&source, &editor).unwrap());
    let pixel = |x: usize, y: usize| &output.pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
    assert_eq!(pixel(11, 12)[2], 255);
    assert!(pixel(11, 12)[0] <= 21);
    assert_eq!(pixel(21, 12)[0], 255);
    assert!(pixel(21, 12)[2] <= 21);
    assert_eq!(pixel(16, 0), &[0, 0, 0, 255]);
}
#[test]
fn prepared_blur_is_path_masked_has_wash_and_restores_exact_history() {
    let _apartment = Apartment::new().unwrap();
    let pixels = (0..32 * 32)
        .flat_map(|i| {
            let value = if i % 32 < 16 { 0 } else { 255 };
            [value, value, value, 255]
        })
        .collect::<Vec<_>>();
    let source = Image {
        width: 32,
        height: 32,
        pixels: pixels.into(),
    };
    let blurred = crate::blur::region(
        &source.pixels,
        (32, 32),
        crate::blur::Region {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        },
        2,
        || false,
    )
    .unwrap();
    let expected = blurred[(16 * 32 + 15) * 4] as f32 * 0.92 + 255.0 * 0.08;
    let image = crate::blur_image::BlurImage::new(32, 32, blurred, 2).unwrap();
    let mut editor = Editor::default();
    editor.set_style(editor.style().with_width(12.0).unwrap());
    editor.begin_blur(Point { x: 8.0, y: 16.0 }, image).unwrap();
    editor.extend(Point { x: 24.0, y: 16.0 }).unwrap();
    assert!(editor.commit());
    let output = tests::decode_png(encode(&source, &editor).unwrap());
    assert!((output.pixels[(16 * 32 + 15) * 4] as f32 - expected).abs() <= 1.0);
    assert_eq!(&output.pixels[..32 * 4], &source.pixels[..32 * 4]);
    assert!(output.pixels[(16 * 32 + 17) * 4] < 255);
    assert!(editor.undo());
    assert_eq!(
        tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        source.pixels
    );
    assert!(editor.redo());
    assert_eq!(
        tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        output.pixels
    );
}
