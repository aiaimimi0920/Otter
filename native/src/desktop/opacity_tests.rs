//! 真实 WIC/D2D 输出验证，不把模型中的透明度数值当作合成正确性证据。
#[path = "annotation_paint_alpha_tests.rs"]
mod paint_alpha;
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, StrokeStyle},
};

fn render(tool: Tool, points: &[(f32, f32)], opacity: f32, copies: usize) -> Image {
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        StrokeStyle::new(8.0, Color(255, 0, 0))
            .unwrap()
            .with_opacity(opacity)
            .unwrap(),
    );
    for _ in 0..copies {
        editor
            .begin(
                tool,
                Point {
                    x: points[0].0,
                    y: points[0].1,
                },
            )
            .unwrap();
        for &(x, y) in &points[1..] {
            editor.extend(Point { x, y }).unwrap();
        }
        assert!(editor.commit());
    }
    super::tests::decode_png(encode(&image, &editor).unwrap())
}

fn alpha(image: &Image, x: usize, y: usize) -> u8 {
    image.pixels[(y * 64 + x) * 4 + 3]
}

#[test]
fn timed_paintbrush_speed_weights_change_exported_width() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let render_speed = |interval: f64| {
        let mut editor = Editor::default();
        editor.set_style(StrokeStyle::new(16.0, Color(255, 0, 0)).unwrap());
        editor
            .begin_timed(Tool::Paintbrush, Point { x: 4.0, y: 32.0 }, 0.0)
            .unwrap();
        for i in 1..8 {
            let point = Point {
                x: 4.0 + i as f32 * 8.0,
                y: 32.0,
            };
            editor
                .extend_timed(point, false, i as f64 * interval, point)
                .unwrap();
        }
        editor.commit_smoothed(50.0);
        super::tests::decode_png(encode(&source, &editor).unwrap())
    };
    let fast = render_speed(4.0);
    let slow = render_speed(80.0);
    let coverage = |image: &Image| {
        image
            .pixels
            .chunks_exact(4)
            .map(|p| p[3] as u64)
            .sum::<u64>()
    };
    assert!(coverage(&slow) > coverage(&fast));
    assert_eq!(alpha(&fast, 32, 32), 255);
    assert_eq!(alpha(&slow, 32, 32), 255);
}

#[test]
fn paintbrush_native_layers_taper_and_do_not_darken_self_intersections() {
    let _apartment = Apartment::new().unwrap();
    let straight = render(Tool::Paintbrush, &[(4.0, 32.0), (60.0, 32.0)], 0.5, 1);
    assert!(alpha(&straight, 32, 32).abs_diff(128) <= 1);
    assert!(alpha(&straight, 32, 34) > 0);
    assert_eq!(alpha(&straight, 59, 34), 0);
    assert!(straight
        .pixels
        .chunks_exact(4)
        .any(|p| p[3] > 0 && p[3] < 120));
    let crossing = render(
        Tool::Paintbrush,
        &[(8.0, 8.0), (56.0, 56.0), (8.0, 56.0), (56.0, 8.0)],
        0.5,
        1,
    );
    assert!(alpha(&crossing, 32, 32).abs_diff(128) <= 1);
    assert!(crossing.pixels.chunks_exact(4).all(|p| p[3] <= 129));
}

#[test]
fn polyline_exports_every_segment_instead_of_endpoint_chord() {
    let _apartment = Apartment::new().unwrap();
    let image = render(
        Tool::Polyline,
        &[(8.0, 8.0), (48.0, 8.0), (48.0, 48.0)],
        1.0,
        1,
    );
    assert_eq!(alpha(&image, 28, 8), 255);
    assert_eq!(alpha(&image, 48, 28), 255);
    assert_eq!(alpha(&image, 28, 28), 0);
}

#[test]
fn optional_line_arrow_matches_dedicated_arrow_pixels_and_restores_plain_line() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        StrokeStyle::new(8.0, Color(255, 0, 0))
            .unwrap()
            .with_opacity(0.5)
            .unwrap()
            .with_arrow(true),
    );
    editor.begin(Tool::Line, Point { x: 4.0, y: 32.0 }).unwrap();
    editor.extend(Point { x: 60.0, y: 32.0 }).unwrap();
    editor.commit();
    let arrow = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert_eq!(
        arrow.pixels,
        render(Tool::Arrow, &[(4.0, 32.0), (60.0, 32.0)], 0.5, 1).pixels
    );
    editor.restyle(0, editor.style().with_arrow(false));
    let plain = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert_eq!(alpha(&plain, 40, 26), 0);
    assert!(alpha(&arrow, 40, 26) > 0);
    editor.undo();
    assert_eq!(
        super::tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        arrow.pixels
    );
}

#[test]
fn curve_arrow_head_is_exported_with_single_opacity_and_style_undo() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        StrokeStyle::new(4.0, Color(255, 0, 0))
            .unwrap()
            .with_opacity(0.5)
            .unwrap()
            .with_arrow(true),
    );
    editor
        .begin(Tool::Bezier, Point { x: 8.0, y: 16.0 })
        .unwrap();
    editor.extend(Point { x: 56.0, y: 16.0 }).unwrap();
    editor.commit();
    let arrow = crate::bezier_arrow::build(&editor.elements()[0].points, 4.0).unwrap();
    let x = (arrow.head.iter().map(|p| p.x).sum::<f32>() / 3.0) as usize;
    let y = (arrow.head.iter().map(|p| p.y).sum::<f32>() / 3.0) as usize;
    let image = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert!(alpha(&image, x, y).abs_diff(128) <= 1);
    assert!(image.pixels.chunks_exact(4).all(|p| p[3] <= 129));
    editor.restyle(0, editor.style().with_arrow(false));
    let plain = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert_ne!(plain.pixels, image.pixels);
    editor.undo();
    assert!(editor.elements()[0].arrow);
    assert_eq!(
        super::tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        image.pixels
    );
}

#[test]
fn edited_multisegment_bezier_exports_and_undo_restores_pixels() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor
        .begin(Tool::Bezier, Point { x: 8.0, y: 16.0 })
        .unwrap();
    editor.extend(Point { x: 32.0, y: 16.0 }).unwrap();
    editor.commit();
    let original = super::tests::decode_png(encode(&source, &editor).unwrap());
    editor
        .extend_bezier(0, Point { x: 56.0, y: 40.0 }, false)
        .unwrap();
    editor
        .move_bezier_handle(0, 4, Point { x: 44.0, y: 40.0 })
        .unwrap();
    let edited = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert_ne!(original.pixels, edited.pixels);
    assert!(edited
        .pixels
        .chunks_exact(4)
        .enumerate()
        .any(|(i, p)| i % 64 > 40 && p[3] > 0));
    editor.undo();
    editor.undo();
    assert_eq!(
        super::tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        original.pixels
    );
    editor.redo();
    editor.redo();
    assert_eq!(
        super::tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        edited.pixels
    );
}

#[test]
fn bezier_exports_curve_not_chord_and_undo_removes_it() {
    let _apartment = Apartment::new().unwrap();
    let source = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor
        .begin(Tool::Bezier, Point { x: 8.0, y: 16.0 })
        .unwrap();
    editor.extend(Point { x: 56.0, y: 16.0 }).unwrap();
    editor.commit();
    let image = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert!(alpha(&image, 32, 25) > 240);
    assert_eq!(alpha(&image, 32, 16), 0);
    editor.undo();
    let empty = super::tests::decode_png(encode(&source, &editor).unwrap());
    assert!(empty.pixels.iter().all(|value| *value == 0));
    editor.redo();
    assert_eq!(
        super::tests::decode_png(encode(&source, &editor).unwrap()).pixels,
        image.pixels
    );
}

#[test]
fn self_crossing_stroke_has_one_opacity_but_separate_strokes_accumulate() {
    let _apartment = Apartment::new().unwrap();
    let points = [(8.0, 8.0), (56.0, 56.0), (8.0, 56.0), (56.0, 8.0)];
    let single = render(Tool::Brush, &points, 0.5, 1);
    assert!(alpha(&single, 32, 32).abs_diff(128) <= 1);
    assert!(alpha(&single, 16, 16).abs_diff(128) <= 1);
    let double = render(Tool::Brush, &points, 0.5, 2);
    assert!(alpha(&double, 32, 32).abs_diff(192) <= 2);
}

#[test]
fn arrow_join_is_not_darker_and_zero_opacity_produces_no_pixels() {
    let _apartment = Apartment::new().unwrap();
    let arrow = render(Tool::Arrow, &[(4.0, 32.0), (60.0, 32.0)], 0.5, 1);
    assert!(alpha(&arrow, 16, 32).abs_diff(128) <= 1);
    assert!(arrow.pixels.chunks_exact(4).all(|pixel| pixel[3] <= 129));
    let empty = render(Tool::Brush, &[(32.0, 32.0)], 0.0, 1);
    assert!(empty.pixels.iter().all(|value| *value == 0));
}

#[test]
fn all_highlighter_strokes_share_one_non_compounding_wash() {
    let _apartment = Apartment::new().unwrap();
    let points = [(8.0, 8.0), (56.0, 56.0), (8.0, 56.0), (56.0, 8.0)];
    let one = render(Tool::Highlighter, &points, 1.0, 1);
    let many = render(Tool::Highlighter, &points, 1.0, 5);
    assert!(alpha(&one, 32, 32).abs_diff(89) <= 1);
    assert!(alpha(&many, 32, 32).abs_diff(89) <= 1);
    assert!(many.pixels.chunks_exact(4).all(|p| p[3] <= 90));
    let lighter = render(Tool::Highlighter, &points, 0.5, 1);
    assert!(alpha(&lighter, 32, 32).abs_diff(45) <= 1);
}

#[test]
fn highlighter_wash_is_below_normal_annotations_regardless_of_creation_order() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(StrokeStyle::new(12.0, Color(0, 0, 255)).unwrap());
    editor
        .begin(Tool::Brush, Point { x: 32.0, y: 32.0 })
        .unwrap();
    editor.commit();
    editor.set_style(StrokeStyle::new(12.0, Color(255, 0, 0)).unwrap());
    editor
        .begin(Tool::Highlighter, Point { x: 32.0, y: 32.0 })
        .unwrap();
    editor.commit();
    let output = super::tests::decode_png(encode(&image, &editor).unwrap());
    let offset = (32 * 64 + 32) * 4;
    assert_eq!(&output.pixels[offset..offset + 4], &[255, 0, 0, 255]);
}

#[test]
fn bounded_highlighter_layer_preserves_rotated_and_clipped_edges() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    for angle in [0.0, 37.0, 90.0] {
        let render = |tool| {
            let mut editor = Editor::default();
            editor.set_style(StrokeStyle::new(12.0, Color(255, 0, 0)).unwrap());
            editor.begin(tool, Point { x: 1.0, y: 2.0 }).unwrap();
            editor.extend(Point { x: 48.0, y: 20.0 }).unwrap();
            editor.commit();
            editor.rotate(0, angle).unwrap();
            super::tests::decode_png(encode(&image, &editor).unwrap())
        };
        let reference = render(Tool::Brush);
        let wash = render(Tool::Highlighter);
        let mut covered = 0;
        for (normal, highlighted) in reference
            .pixels
            .chunks_exact(4)
            .zip(wash.pixels.chunks_exact(4))
        {
            let expected = (normal[3] as f32 * 0.35).round() as u8;
            assert!(highlighted[3].abs_diff(expected) <= 1, "angle={angle}");
            covered += usize::from(highlighted[3] > 0);
        }
        assert!(covered > 100);
    }
}
