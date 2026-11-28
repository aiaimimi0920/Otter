//! Actual model transactions and field-visibility regressions for secondary bars, using local synthetic images only.
use super::*;
use crate::{editor::Tool, viewport::Point};
#[test]
fn all_property_icons_use_renderable_original_svg() {
    let mut c = fixture();
    for t in [
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Polygon,
        Tool::Brush,
        Tool::Paintbrush,
        Tool::Line,
        Tool::Serial,
        Tool::Text,
        Tool::Mosaic,
        Tool::Blur,
        Tool::Eraser,
    ] {
        c.select_tool(t);
        for f in fields::fields(&c) {
            // Baseline effect parameters must show their tool icons; do not treat None as renderable and skip verification.
            let expected = match f.id {
                CELL => Some(include_str!("../../assets/edit-strip/MosaicTool.svg")),
                STRENGTH => Some(include_str!("../../assets/edit-strip/BlurTool.svg")),
                _ => None,
            };
            if let Some(svg) = expected {
                assert_eq!(f.icon, Some(neuro_mini_ui::Icon::svg(svg)), "{}", f.label);
            }
            if let Some(icon) = f.icon {
                icon.validate().unwrap();
            }
        }
    }
}
fn fixture() -> Canvas {
    let mut c = Canvas::new(Some(super::super::image::Image {
        width: 64,
        height: 64,
        pixels: vec![255; 64 * 64 * 4].into(),
    }))
    .unwrap();
    c.pinned = true;
    c
}
fn ids(c: &Canvas) -> Vec<u16> {
    fields::fields(c).iter().map(|f| f.id).collect()
}
#[test]
fn creation_fields_match_real_tool_and_never_add_crop_or_generic_selection_properties() {
    let mut c = fixture();
    for t in [Tool::Pan, Tool::Select] {
        c.select_tool(t);
        assert!(ids(&c).is_empty());
    }
    c.select_tool(Tool::Rectangle);
    assert_eq!(
        ids(&c),
        vec![FILL, STROKE, WIDTH, RADIUS, DASH, RATIO, STEP]
    );
    c.select_tool(Tool::Ellipse);
    assert!(!ids(&c).contains(&RADIUS));
    c.select_tool(Tool::Polygon);
    assert!(ids(&c).contains(&SIDES));
    assert!(!ids(&c).contains(&RATIO));
    c.select_tool(Tool::Line);
    assert!(ids(&c).contains(&ANGLE));
    c.select_tool(Tool::Bezier);
    assert!(!ids(&c).contains(&ANGLE));
    c.select_tool(Tool::Brush);
    assert!(ids(&c).contains(&GLOW));
    c.select_tool(Tool::Paintbrush);
    assert!(!ids(&c).contains(&GLOW));
    assert!(ids(&c).contains(&SMOOTH));
    c.select_tool(Tool::Serial);
    assert_eq!(ids(&c), vec![FILL, STROKE, SERIAL_RADIUS, FONT]);
    c.crop_mode = true;
    assert!(ids(&c).is_empty());
}
#[test]
fn exact_defaults_keep_profile_ranges_and_independent_smoothing() {
    unsafe {
        let mut c = fixture();
        c.select_tool(Tool::Rectangle);
        let stamp = c.editor.document_stamp();
        apply::value(&mut c, WIDTH, Some("999")).unwrap();
        apply::value(&mut c, RADIUS, Some("999")).unwrap();
        c.select_tool(Tool::Brush);
        apply::value(&mut c, SMOOTH, Some("37")).unwrap();
        apply::value(&mut c, OPACITY, Some("50")).unwrap();
        c.select_tool(Tool::Rectangle);
        assert_eq!(c.editor.style().width(), 999.0);
        assert_eq!(c.editor.style().corner_radius(), 999.0);
        apply::value(&mut c, WIDTH, Some("0")).unwrap();
        assert_eq!(c.editor.style().width(), 0.0);
        c.select_tool(Tool::Brush);
        assert_eq!(c.tool_profiles.smoothing(Tool::Brush), 37);
        assert_eq!(c.editor.style().opacity(), 0.5);
        assert!(stamp.matches(&c.editor));
        assert!(apply::value(&mut c, WIDTH, Some("1000")).is_err());
    }
}
#[test]
fn rectangle_and_ellipse_use_the_original_paired_ratio_control_and_toggle() {
    unsafe {
        let mut c = fixture();
        for tool in [Tool::Rectangle, Tool::Ellipse] {
            c.select_tool(tool);
            let stamp = c.editor.document_stamp();
            apply::value(&mut c, RATIO, Some("16:9")).unwrap();
            let ratio = fields::fields(&c)
                .into_iter()
                .find(|f| f.id == RATIO)
                .unwrap();
            assert!(ratio.icon.is_some());
            assert!(
                matches!(ratio.kind, neuro_mini_ui::PropertyKind::Ratio { ref value, active: true } if value == "16:9")
            );
            assert!(!ids(&c).contains(&CONSTRAIN));
            apply::value(&mut c, RATIO, None).unwrap();
            assert!(!c.tool_profiles.shapes.lock_aspect);
            assert_eq!(c.tool_profiles.shapes.ratio_label(), "16:9");
            apply::value(&mut c, RATIO, None).unwrap();
            assert!(c.tool_profiles.shapes.lock_aspect);
            assert!(apply::value(&mut c, RATIO, Some("0:9")).is_err());
            assert_eq!(c.tool_profiles.shapes.ratio_label(), "16:9");
            assert!(stamp.matches(&c.editor));
        }
    }
}
#[test]
fn selected_line_properties_change_object_not_defaults_and_arrow_can_be_disabled() {
    unsafe {
        let mut c = fixture();
        c.editor
            .begin(Tool::Arrow, Point { x: 4.0, y: 4.0 })
            .unwrap();
        c.editor.extend(Point { x: 40.0, y: 40.0 }).unwrap();
        c.editor.commit();
        c.select_tool(Tool::Select);
        c.selected = Some((0, c.editor.revision()));
        let default = c.editor.style();
        assert!(!ids(&c).contains(&ANGLE));
        apply::value(&mut c, WIDTH, Some("9")).unwrap();
        assert_eq!(c.editor.elements()[0].width, 9.0);
        assert_eq!(c.editor.style(), default);
        assert_eq!(c.selected_index(), Some(0));
        apply::value(&mut c, ARROW, None).unwrap();
        assert!(!c.editor.elements()[0].has_straight_arrow());
        assert!(c.editor.undo());
        assert!(c.editor.elements()[0].has_straight_arrow());
        assert_eq!(c.editor.elements()[0].tool, Tool::Arrow);
        assert!(c.editor.redo());
        assert!(!c.editor.elements()[0].has_straight_arrow());
    }
}
#[test]
fn selected_text_and_serial_size_font_and_color_are_history_transactions() {
    unsafe {
        let mut c = fixture();
        c.editor
            .insert_serial(Point { x: 30.0, y: 30.0 }, 14.0, "微软雅黑")
            .unwrap();
        c.select_tool(Tool::Select);
        c.selected = Some((0, c.editor.revision()));
        let target = MenuTarget::capture(&c);
        let default = c.tool_profiles.serial_radius();
        let original = c.editor.elements()[0].clone();
        apply::value(&mut c, SERIAL_RADIUS, Some("28")).unwrap();
        assert!(!target.matches(&c));
        assert_eq!(c.selected_index(), Some(0));
        let p = &c.editor.elements()[0].points;
        assert_eq!((p[1].x - p[0].x) / 2.0, 28.0);
        assert_eq!(c.tool_profiles.serial_radius(), default);
        apply::font(&mut c, "Consolas", None).unwrap();
        assert_eq!(
            c.editor.elements()[0].text.as_ref().unwrap().family(),
            "Consolas"
        );
        apply::color(&mut c, false, Some(crate::stroke_style::Color(1, 2, 3)));
        assert_eq!(
            c.editor.elements()[0].color,
            crate::stroke_style::Color(1, 2, 3)
        );
        assert_eq!(c.editor.serial_count(), 1);
        for _ in 0..3 {
            assert!(c.editor.undo());
        }
        assert_eq!(c.editor.elements()[0].points, original.points);
        assert_eq!(c.editor.elements()[0].text, original.text);
    }
}

#[test]
fn transparent_paints_are_available_for_defaults_and_single_selection_with_undo() {
    use crate::stroke_style::Color;
    let mut c = fixture();
    c.select_tool(Tool::Rectangle);
    let stamp = c.editor.document_stamp();
    apply::color(&mut c, true, Some(Color(1, 2, 3)));
    apply::color(&mut c, true, None);
    apply::color(&mut c, false, None);
    assert_eq!(c.editor.style().fill(), None);
    assert_eq!(c.editor.style().paint_alpha().stroke(), 0.0);
    assert!(stamp.matches(&c.editor));
    c.editor
        .insert_serial(Point { x: 30.0, y: 30.0 }, 14.0, "微软雅黑")
        .unwrap();
    c.select_tool(Tool::Select);
    c.selected = Some((0, c.editor.revision()));
    let original = c.editor.elements()[0].style();
    let defaults = c.editor.style();
    apply::color(&mut c, true, None);
    assert_eq!(c.editor.elements()[0].fill, None);
    assert_eq!(c.editor.style(), defaults);
    assert_eq!(c.selected_index(), Some(0));
    assert_ne!(c.editor.elements()[0].style(), original);
    assert!(c.editor.undo());
    assert_eq!(c.editor.elements()[0].style(), original);
}
#[test]
fn font_trigger_is_original_icon_dropdown_and_text_serial_defaults_are_independent() {
    let mut c = fixture();
    c.select_tool(Tool::Text);
    let font = fields::fields(&c)
        .into_iter()
        .find(|f| f.id == FONT)
        .unwrap();
    assert!(font.icon.is_some());
    assert!(matches!(
        font.kind,
        neuro_mini_ui::PropertyKind::Dropdown { width: 110, .. }
    ));
    let stamp = c.editor.document_stamp();
    unsafe {
        apply::font(&mut c, "Consolas", None).unwrap();
    }
    assert_eq!(c.tool_profiles.text_font.family(), "Consolas");
    c.select_tool(Tool::Serial);
    unsafe {
        apply::font(&mut c, "Arial", None).unwrap();
    }
    assert_eq!(c.tool_profiles.serial_font_family(), "Arial");
    assert_eq!(c.tool_profiles.text_font.family(), "Consolas");
    assert!(stamp.matches(&c.editor));
}

#[test]
fn brush_glow_is_a_remembered_property_not_a_tool_switch_or_document_edit() {
    unsafe {
        let mut c = fixture();
        c.select_tool(Tool::Brush);
        apply::value(&mut c, WIDTH, Some("19")).unwrap();
        apply::value(&mut c, OPACITY, Some("50")).unwrap();
        apply::value(&mut c, SMOOTH, Some("100")).unwrap();
        let defaults = c.editor.style();
        let stamp = c.editor.document_stamp();
        let fields_before = ids(&c);
        apply::value(&mut c, GLOW, None).unwrap();
        assert_eq!(c.tool, Tool::Brush);
        assert_eq!(c.editor.style(), defaults);
        assert_eq!(ids(&c), fields_before);
        assert!(fields::fields(&c).iter().any(|f| f.id == GLOW
            && matches!(
                f.kind,
                neuro_mini_ui::PropertyKind::Action { active: true, .. }
            )));
        assert_eq!(c.tool_profiles.smoothing(Tool::Brush), 100);
        assert!(stamp.matches(&c.editor));
        for t in [
            Tool::Paintbrush,
            Tool::Highlighter,
            Tool::Select,
            Tool::Brush,
        ] {
            c.select_tool(t);
            assert!(c.tool_profiles.brush_highlighter_enabled());
            if t == Tool::Highlighter {
                assert_eq!(c.editor.style().opacity(), 1.0);
                assert!(!ids(&c).contains(&OPACITY));
                assert!(!ids(&c).contains(&SMOOTH));
            }
        }
        assert_eq!(c.editor.style(), defaults);
        apply::value(&mut c, GLOW, None).unwrap();
        assert!(!c.tool_profiles.brush_highlighter_enabled());
        assert_eq!(c.tool, Tool::Brush);
        assert_eq!(c.editor.style(), defaults);
        assert!(stamp.matches(&c.editor));
    }
}

#[test]
fn brush_glow_reaches_draft_commit_history_and_existing_session_format() {
    unsafe {
        let mut c = fixture();
        c.viewport.origin = Point::default();
        c.select_tool(Tool::Brush);
        apply::value(&mut c, OPACITY, Some("50")).unwrap();
        apply::value(&mut c, SMOOTH, Some("100")).unwrap();
        apply::value(&mut c, GLOW, None).unwrap();
        let p = |x, y| Point { x, y };
        assert!(c.pointer_down(p(0., 0.)));
        assert!(c.pointer_move(p(10., 10.)));
        assert_eq!(c.editor.draft().unwrap().tool, Tool::Highlighter);
        assert_eq!(c.editor.draft().unwrap().opacity, 0.5);
        assert_eq!(c.editor.draft().unwrap().points[1].y, 10.);
        assert!(c.pointer_up(p(20., 0.)));
        assert_eq!(c.editor.elements()[0].points[1].y, 1.25);
        let bytes =
            crate::session_vector_write::encode(&c.editor, None, |_| unreachable!()).unwrap();
        let raw =
            serde_json::value::RawValue::from_string(String::from_utf8(bytes).unwrap()).unwrap();
        let restored = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
        assert_eq!(restored.elements()[0].tool, Tool::Highlighter);
        assert_eq!(restored.elements()[0].opacity, 0.5);
        assert_eq!(restored.elements()[0].points, c.editor.elements()[0].points);
        assert!(c.editor.undo());
        assert!(!c.editor.undo());
        assert!(c.editor.redo());
        assert_eq!(c.editor.elements()[0].points, restored.elements()[0].points);
        apply::value(&mut c, GLOW, None).unwrap();
        c.pointer_down(p(0., 10.));
        c.pointer_up(p(20., 10.));
        assert_eq!(c.editor.elements()[1].tool, Tool::Brush);
        assert_eq!(c.editor.elements()[1].opacity, 0.5);
    }
}

#[test]
fn spotlight_and_magnifier_expose_shape_caret_color_and_whole_source_box_drag() {
    use crate::stroke_style::Color;
    for cmd in [0x21f1usize, 0x21f2, 0x21f3, 0x21f4] {
        super::super::tool_panel::icons_for_test(cmd)
            .validate()
            .unwrap();
    }
    for idx in 0..3 {
        fields::shape_icon(idx).validate().unwrap();
        fields::shape_button_icon(idx).validate().unwrap();
    }
    let mut c = fixture();
    c.viewport.origin = Point::default();
    for tool in [Tool::SpotRect, Tool::Magnifier] {
        c.select_tool(tool);
        let list = fields::fields(&c);
        let shape = list.iter().find(|f| f.id == SHAPE).unwrap();
        assert!(shape.icon.is_some());
        assert!(matches!(
            &shape.kind,
            neuro_mini_ui::PropertyKind::Action { value, width: 28, .. } if value.is_empty()
        ));
    }
    c.select_tool(Tool::Magnifier);
    assert!(ids(&c).contains(&STROKE));
    apply::color(&mut c, false, Some(Color(16, 185, 129)));
    c.select_tool(Tool::Rectangle);
    assert_ne!(c.editor.style().color(), Color(16, 185, 129));
    c.select_tool(Tool::Magnifier);
    assert_eq!(c.editor.style().color(), Color(16, 185, 129));
    // Draw a 50x50 magnifier inside the 64x64 image at (10, 10)..(60, 60); center is (35, 35), 250% source box is (25, 25)..(45, 45).
    assert!(c.pointer_down(Point { x: 10.0, y: 10.0 }));
    assert!(c.pointer_move(Point { x: 60.0, y: 60.0 }));
    assert!(c.pointer_up(Point { x: 60.0, y: 60.0 }));
    assert_eq!(c.editor.elements()[0].color, Color(16, 185, 129));
    // Grab near the bottom-right interior of the source box (43, 43) — 11.3px away from center (35, 35) — and drag by (+15, +10).
    assert!(c.pointer_down(Point { x: 43.0, y: 43.0 }));
    assert!(c.pointer_move(Point { x: 58.0, y: 53.0 }));
    assert!(c.pointer_up(Point { x: 58.0, y: 53.0 }));
    let [src, _] = c.editor.elements()[0].lens_handles().unwrap();
    assert!(
        (src.x - 50.0).abs() < 1e-3 && (src.y - 45.0).abs() < 1e-3,
        "expected smooth source center (50, 45), got {src:?}"
    );
}
