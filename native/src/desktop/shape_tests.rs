//! 原生形状像素回归：填充、边框、圆角及整组透明度。
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, StrokeStyle},
};

fn render(tool: Tool, filled: bool, radius: f32, reverse: bool) -> Image {
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let style = StrokeStyle::new(4.0, Color(255, 0, 0))
        .unwrap()
        .with_fill(filled.then_some(Color(0, 0, 255)))
        .with_corner_radius(radius)
        .unwrap()
        .with_opacity(0.5)
        .unwrap();
    let mut editor = Editor::default();
    editor.set_style(style);
    let a = Point { x: 8.0, y: 8.0 };
    let b = Point { x: 56.0, y: 56.0 };
    editor.begin(tool, if reverse { b } else { a }).unwrap();
    editor.extend(if reverse { a } else { b }).unwrap();
    assert!(editor.commit());
    super::tests::decode_png(encode(&image, &editor).unwrap())
}

fn pixel(image: &Image, x: usize, y: usize) -> &[u8] {
    &image.pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4]
}

#[test]
fn filled_shapes_keep_separate_border_color_and_group_opacity() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Rectangle, Tool::RoundedRectangle, Tool::Ellipse] {
        let image = render(tool, true, 12.0, false);
        let center = pixel(&image, 32, 32);
        assert!(center[0].abs_diff(128) <= 1);
        assert_eq!(center[2], 0);
        let border = pixel(&image, 32, 8);
        assert!(border[2].abs_diff(128) <= 1);
        assert!(image.pixels.chunks_exact(4).all(|p| p[3] <= 129));
        assert_eq!(render(tool, true, 12.0, true).pixels, image.pixels);
        assert_eq!(pixel(&render(tool, false, 12.0, false), 32, 32)[3], 0);
    }
}

#[test]
fn rounded_rectangle_cuts_corners_and_clamps_oversized_radius() {
    let _apartment = Apartment::new().unwrap();
    let rounded = render(Tool::RoundedRectangle, true, 12.0, false);
    assert_eq!(pixel(&rounded, 8, 8)[3], 0);
    assert!(pixel(&render(Tool::Rectangle, true, 12.0, false), 8, 8)[3] > 0);
    assert_eq!(
        render(Tool::RoundedRectangle, true, 24.0, false).pixels,
        render(Tool::RoundedRectangle, true, 256.0, false).pixels
    );
}

#[test]
fn polygon_shapes_fill_center_preserve_empty_corners_and_reverse_drag() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Triangle, Tool::Polygon] {
        for radius in [0.0, 12.0, 256.0] {
            let image = render(tool, true, radius, false);
            let center = pixel(&image, 32, 32);
            assert!(
                center[0].abs_diff(128) <= 1,
                "tool={tool:?}, radius={radius}"
            );
            assert_eq!(pixel(&image, 8, 8)[3], 0);
            assert!(image.pixels.chunks_exact(4).all(|p| p[3] <= 129));
            assert_eq!(image.pixels, render(tool, true, radius, true).pixels);
        }
    }
}

#[test]
fn resized_shape_exports_scaled_geometry_and_undo_restores_pixels() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(StrokeStyle::default().with_fill(Some(Color(0, 0, 255))));
    editor
        .begin(Tool::Rectangle, Point { x: 20.0, y: 20.0 })
        .unwrap();
    editor.extend(Point { x: 40.0, y: 40.0 }).unwrap();
    editor.commit();
    let before = super::tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(pixel(&before, 15, 30)[3], 0);
    editor.resize(0, 2.0).unwrap();
    let after = super::tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(pixel(&after, 15, 30), &[255, 0, 0, 255]);
    editor.undo();
    assert_eq!(
        super::tests::decode_png(encode(&image, &editor).unwrap()).pixels,
        before.pixels
    );
}

#[test]
fn rotated_rectangle_exports_rotated_fill_and_restores_target_for_next_stroke() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        StrokeStyle::default()
            .with_fill(Some(Color(0, 0, 255)))
            .with_opacity(0.5)
            .unwrap(),
    );
    editor
        .begin(Tool::Rectangle, Point { x: 12.0, y: 24.0 })
        .unwrap();
    editor.extend(Point { x: 52.0, y: 40.0 }).unwrap();
    editor.commit();
    editor.rotate(0, 90.0).unwrap();
    let rotated = super::tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(pixel(&rotated, 16, 32)[3], 0);
    assert!(pixel(&rotated, 32, 16)[0].abs_diff(128) <= 1);
    editor.set_style(StrokeStyle::new(8.0, Color(255, 0, 0)).unwrap());
    editor.begin(Tool::Brush, Point { x: 8.0, y: 8.0 }).unwrap();
    editor.commit();
    let next = super::tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(pixel(&next, 8, 8), &[0, 0, 255, 255]);
    editor.undo();
    editor.undo();
    let original = super::tests::decode_png(encode(&image, &editor).unwrap());
    assert!(pixel(&original, 16, 32)[0].abs_diff(128) <= 1);
    assert_eq!(pixel(&original, 32, 16)[3], 0);
}

#[test]
fn axis_resized_ellipse_changes_only_requested_geometry_axis() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(StrokeStyle::default().with_fill(Some(Color(0, 0, 255))));
    editor
        .begin(Tool::Ellipse, Point { x: 22.0, y: 22.0 })
        .unwrap();
    editor.extend(Point { x: 42.0, y: 42.0 }).unwrap();
    editor.commit();
    editor.resize_axes(0, Point { x: 2.0, y: 1.0 }).unwrap();
    let output = super::tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(pixel(&output, 16, 32), &[255, 0, 0, 255]);
    assert_eq!(pixel(&output, 32, 16)[3], 0);
    assert_eq!(pixel(&output, 32, 32), &[255, 0, 0, 255]);
}
