//! 校验真实虚线像素，尤其是 D2D 线宽单位与 SVG 图像单位的转换。
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, DashPattern, StrokeStyle},
};

fn render(tool: Tool, width: f32, dash: DashPattern) -> Image {
    let image = Image {
        width: 64,
        height: 64,
        pixels: vec![0; 64 * 64 * 4].into(),
    };
    let mut editor = Editor::default();
    editor.set_style(
        StrokeStyle::new(width, Color(255, 0, 0))
            .unwrap()
            .with_dash(dash)
            .with_fill(Some(Color(0, 0, 255))),
    );
    let start = Point {
        x: 4.0,
        y: if tool == Tool::Rectangle { 8.0 } else { 32.0 },
    };
    let end = Point {
        x: 60.0,
        y: if tool == Tool::Rectangle { 56.0 } else { 32.0 },
    };
    editor.begin(tool, start).unwrap();
    editor.extend(end).unwrap();
    assert!(editor.commit());
    super::tests::decode_png(encode(&image, &editor).unwrap())
}

#[test]
fn dash_spacing_is_independent_of_stroke_width() {
    let _apartment = Apartment::new().unwrap();
    for pattern in [DashPattern::Dash, DashPattern::DashDot] {
        let thin = render(Tool::Line, 2.0, pattern);
        let thick = render(Tool::Line, 10.0, pattern);
        for x in 4..60 {
            let a = thin.pixels[(32 * 64 + x) * 4 + 3];
            let b = thick.pixels[(32 * 64 + x) * 4 + 3];
            assert!(a.abs_diff(b) <= 1, "x={x}, pattern={pattern:?}: {a} != {b}");
            let period = if pattern == DashPattern::Dash { 12 } else { 9 };
            let offset = (x - 4) % period;
            let on = if pattern == DashPattern::Dash {
                offset < 8
            } else {
                offset < 4 || offset == 6
            };
            assert_eq!(a, if on { 255 } else { 0 }, "x={x}, pattern={pattern:?}");
        }
    }
}

#[test]
fn dashed_border_does_not_punch_holes_in_shape_fill() {
    let _apartment = Apartment::new().unwrap();
    let image = render(Tool::Rectangle, 4.0, DashPattern::DashDot);
    for x in 8..56 {
        let offset = (32 * 64 + x) * 4;
        assert_eq!(&image.pixels[offset..offset + 4], &[255, 0, 0, 255]);
    }
}

#[test]
fn brush_dash_phase_does_not_restart_at_sample_points() {
    let _apartment = Apartment::new().unwrap();
    for pattern in [DashPattern::Dash, DashPattern::DashDot] {
        let source = Image {
            width: 64,
            height: 64,
            pixels: vec![0; 64 * 64 * 4].into(),
        };
        for width in [2.0, 10.0] {
            let mut editor = Editor::default();
            editor.set_style(
                StrokeStyle::new(width, Color(255, 0, 0))
                    .unwrap()
                    .with_dash(pattern),
            );
            editor
                .begin(Tool::Brush, Point { x: 4.0, y: 32.0 })
                .unwrap();
            for x in [9.0, 17.0, 22.0, 37.0, 49.0, 60.0] {
                editor.extend(Point { x, y: 32.0 }).unwrap();
            }
            editor.commit();
            let dense = super::tests::decode_png(encode(&source, &editor).unwrap());
            let line = render(Tool::Line, width, pattern);
            assert_eq!(
                dense.pixels, line.pixels,
                "pattern={pattern:?}, width={width}"
            );
        }
    }
}

#[test]
fn highlighter_and_polyline_keep_solid_round_paths_despite_dash_setting() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Highlighter, Tool::Polyline] {
        let solid = render(tool, 8.0, DashPattern::Solid);
        let dashed = render(tool, 8.0, DashPattern::Dash);
        assert_eq!(solid.pixels, dashed.pixels, "{tool:?}");
        assert!(
            solid.pixels[(32 * 64 + 2) * 4 + 3] > 0,
            "round starting cap"
        );
    }
}
