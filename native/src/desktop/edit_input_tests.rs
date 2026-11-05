//! 使用真实 Canvas 状态验证宿主映射，图像内容为内存夹具，不读用户数据。
use super::{canvas::Canvas, image::Image};
use crate::{editor::Tool, viewport::Point};

fn canvas() -> Canvas {
    Canvas::new(Some(Image {
        width: 100,
        height: 80,
        pixels: vec![255; 100 * 80 * 4].into(),
    }))
    .unwrap()
}
fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

#[test]
fn mosaic_input_commits_continuous_path_and_restores_independent_width() {
    let mut canvas = canvas();
    assert_eq!(
        super::tool_commands::tool_for_command(0x4D),
        Some(Tool::Mosaic)
    );
    canvas.select_tool(Tool::Brush);
    canvas
        .editor
        .set_style(canvas.editor.style().with_width(7.0).unwrap());
    canvas.select_tool(Tool::Mosaic);
    assert_eq!(canvas.editor.style().width(), 28.0);
    canvas
        .editor
        .set_style(canvas.editor.style().with_mosaic_cell(24).unwrap());
    assert!(canvas.pointer_down(p(30.0, 30.0)));
    canvas.pointer_move(p(40.0, 35.0));
    canvas.pointer_move(p(50.0, 45.0));
    canvas.pointer_up(p(60.0, 50.0));
    assert_eq!(canvas.editor.elements().len(), 1);
    let stroke = canvas.editor.elements()[0].clone();
    assert_eq!(stroke.tool, Tool::Mosaic);
    assert_eq!(stroke.width, 28.0);
    assert_eq!(stroke.mosaic_cell, 24.0);
    assert_eq!(stroke.points.len(), 4);
    canvas.select_tool(Tool::Select);
    canvas.pointer_down(p(22.0, 30.0));
    assert_eq!(canvas.selected_index(), Some(0));
    canvas.cancel_gesture();
    canvas.select_tool(Tool::Mosaic);
    canvas
        .editor
        .set_style(canvas.editor.style().with_width(44.0).unwrap());
    canvas.select_tool(Tool::Brush);
    assert_eq!(canvas.editor.style().width(), 7.0);
    canvas.select_tool(Tool::Mosaic);
    assert_eq!(canvas.editor.style().width(), 44.0);
    assert_eq!(canvas.editor.style().mosaic_cell(), 24.0);
    canvas.pointer_down(p(35.0, 35.0));
    assert!(canvas.cancel_gesture());
    assert_eq!(canvas.editor.elements().len(), 1);
    assert!(canvas.editor.undo());
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.editor.redo());
    assert_eq!(canvas.editor.elements()[0].points, stroke.points);
    canvas.pointer_down(p(45.0, 45.0));
    canvas.pointer_up(p(45.0, 45.0));
    assert_eq!(canvas.editor.elements()[1].points.len(), 1);
    assert_eq!(canvas.editor.elements()[1].width, 44.0);
}

#[test]
fn serial_creation_uses_configured_radius_and_independent_fill() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Serial);
    canvas.tool_profiles.set_serial_radius(30);
    canvas.tool_profiles.set_serial_font_family("Segoe UI");
    let style = canvas
        .editor
        .style()
        .with_color(crate::stroke_style::Color(1, 2, 3))
        .with_fill(None);
    canvas.editor.set_style(style);
    canvas.pointer_down(p(60.0, 60.0));
    canvas.pointer_up(p(60.0, 60.0));
    let serial = &canvas.editor.elements()[0];
    assert_eq!(serial.points, vec![p(10.0, 10.0), p(70.0, 70.0)]);
    assert_eq!(serial.fill, None);
    assert_eq!(serial.color, crate::stroke_style::Color(1, 2, 3));
    assert_eq!(serial.text.as_ref().unwrap().font_size(), 35.0);
    assert_eq!(serial.text.as_ref().unwrap().family(), "Segoe UI");
    let revision = canvas.editor.revision();
    canvas.tool_profiles.set_serial_font_family("微软雅黑");
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(
        canvas.editor.elements()[0].text.as_ref().unwrap().family(),
        "Segoe UI"
    );
    canvas.select_tool(Tool::Brush);
    canvas.select_tool(Tool::Serial);
    assert_eq!(canvas.editor.style(), style);
    assert_eq!(canvas.editor.elements().len(), 1);
}

#[test]
fn serial_click_commits_once_and_undo_reuses_number() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Serial);
    canvas.pointer_down(p(50.0, 50.0));
    canvas.pointer_up(p(50.0, 50.0));
    assert_eq!(canvas.editor.elements().len(), 1);
    assert_eq!(
        canvas.editor.elements()[0].text.as_ref().unwrap().content(),
        "1"
    );
    canvas.editor.undo();
    canvas.pointer_down(p(60.0, 60.0));
    canvas.pointer_up(p(60.0, 60.0));
    assert_eq!(
        canvas.editor.elements()[0].text.as_ref().unwrap().content(),
        "1"
    );
}

#[test]
fn persistent_shape_constraints_survive_tool_switch_without_history_changes() {
    let mut canvas = canvas();
    canvas.tool_profiles.shapes.lock_aspect = true;
    canvas.tool_profiles.shapes.cycle_ratio(); // 4:3
    canvas.tool_profiles.shapes.cycle_step(); // 5 px
    canvas.select_tool(Tool::Rectangle);
    assert!(!canvas.editor.has_edit_state());
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_move(p(70.0, 40.0));
    assert_eq!(canvas.editor.draft().unwrap().points[1], p(50.0, 40.0));
    canvas.refresh_constraints(true, false);
    assert_eq!(canvas.editor.draft().unwrap().points[1], p(50.0, 50.0));
    canvas.pointer_up(p(70.0, 40.0));
    assert_eq!(canvas.editor.elements()[0].points[1], p(50.0, 40.0));
    canvas.select_tool(Tool::Brush);
    canvas.select_tool(Tool::Ellipse);
    assert!(canvas.tool_profiles.shapes.lock_aspect);
    assert_eq!(canvas.tool_profiles.shapes.ratio_label(), "4:3");
    assert!(canvas.editor.undo());
    assert!(!canvas.editor.undo());
    assert!(canvas.editor.redo());
    assert_eq!(canvas.editor.elements()[0].points[1], p(50.0, 40.0));
}

#[test]
fn explicit_message_time_drives_paintbrush_and_modifier_refresh_reuses_last_sample_time() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Paintbrush);
    canvas.pointer_down_at(p(30.0, 30.0), 100.0);
    canvas.pointer_move_at(p(60.0, 50.0), false, false, 110.0);
    let weight = canvas.editor.draft().unwrap().brush_weights[1];
    let expected = crate::brush_dynamics::sample(
        p(40.0, 30.0),
        110.0,
        Some(crate::brush_dynamics::sample(p(10.0, 10.0), 100.0, None)),
    )
    .weight;
    assert_eq!(weight, expected);
    canvas.refresh_constraints(true, false);
    assert_eq!(canvas.last_pointer_time, 110.0);
    assert_eq!(canvas.editor.draft().unwrap().brush_weights[1], weight);
    canvas.pointer_up_at(p(70.0, 55.0), false, false, 120.0);
    assert_eq!(canvas.editor.elements()[0].brush_weights.len(), 3);
}

#[test]
fn brush_smoothing_setting_is_applied_only_when_pointer_gesture_commits() {
    let mut canvas = canvas();
    for _ in 0..4 {
        canvas.tool_profiles.cycle_brush_smoothing();
    }
    canvas.select_tool(Tool::Brush);
    canvas.pointer_down(p(20.0, 20.0));
    canvas.pointer_move(p(30.0, 30.0));
    canvas.pointer_up(p(40.0, 20.0));
    assert_eq!(canvas.editor.elements()[0].points[1], p(10.0, 1.25));
    canvas.select_tool(Tool::Line);
    canvas.select_tool(Tool::Brush);
    assert_eq!(canvas.tool_profiles.brush_smoothing(), 100);
    canvas.tool_profiles.cycle_brush_smoothing();
    assert_eq!(canvas.tool_profiles.brush_smoothing(), 0);
    assert_eq!(canvas.editor.elements()[0].points[1], p(10.0, 1.25));
}

#[test]
fn linear_modifiers_use_image_coordinates_and_release_state() {
    for tool in [Tool::Line, Tool::Arrow] {
        let mut canvas = canvas();
        canvas.viewport.scale = 2.0;
        canvas.select_tool(tool);
        canvas.pointer_down(p(40.0, 60.0)); // 图像起点 (10,20)。
        canvas.pointer_move_modified(p(100.0, 100.0), true, true);
        assert_eq!(canvas.editor.draft().unwrap().points[1], p(40.0, 50.0));
        assert!(canvas.editor.elements().is_empty());
        canvas.pointer_up_modified(p(110.0, 110.0), false, true);
        assert_eq!(
            canvas.editor.elements()[0].points,
            vec![p(10.0, 20.0), p(50.0, 50.0)]
        );
        assert!(canvas.editor.undo());
        assert!(canvas.editor.elements().is_empty());
        assert!(!canvas.editor.undo());
        assert!(canvas.editor.redo());
    }
}

#[test]
fn control_step_does_not_change_freehand_points_or_cancel_semantics() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Brush);
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_move_modified(p(47.0, 43.0), false, true);
    assert_eq!(canvas.editor.draft().unwrap().points[1], p(27.0, 23.0));
    canvas.cancel_gesture();
    canvas.pointer_up_modified(p(60.0, 60.0), true, true);
    assert!(canvas.editor.elements().is_empty());
    assert!(!canvas.editor.undo());
}

#[test]
fn shift_converts_brush_and_highlighter_to_angle_locked_segment() {
    for tool in [Tool::Brush, Tool::Highlighter] {
        let mut canvas = canvas();
        canvas.select_tool(tool);
        canvas.pointer_down(p(30.0, 30.0));
        canvas.pointer_move(p(40.0, 50.0));
        canvas.pointer_move(p(60.0, 50.0));
        assert_eq!(canvas.editor.draft().unwrap().points.len(), 3);
        assert!(canvas.refresh_constraints(true, true));
        let points = &canvas.editor.draft().unwrap().points;
        assert_eq!(points.len(), 2);
        assert!(((points[1].x - 10.0) - (points[1].y - 10.0)).abs() < 0.001);
        // Ctrl 不将画笔端点进一步量化，松开 Shift 后只在指针事件继续采样。
        assert!((points[1].x - 35.495).abs() < 0.001);
        assert!(!canvas.refresh_constraints(false, false));
        canvas.pointer_up_modified(p(70.0, 55.0), false, false);
        assert_eq!(canvas.editor.elements()[0].points.len(), 3);
        assert_eq!(canvas.editor.elements()[0].points[2], p(50.0, 35.0));
        assert!(canvas.editor.undo());
        assert!(!canvas.editor.undo());
    }
}

#[test]
fn shape_modifiers_preserve_square_at_edges_and_commit_final_state() {
    for tool in [
        Tool::Rectangle,
        Tool::RoundedRectangle,
        Tool::Ellipse,
        Tool::Triangle,
        Tool::Polygon,
    ] {
        let mut canvas = canvas();
        canvas.viewport.scale = 2.0;
        canvas.select_tool(tool);
        canvas.pointer_down(p(180.0, 100.0)); // 图像 (80,40)。
        canvas.pointer_move_modified(p(280.0, 160.0), true, true);
        assert_eq!(canvas.editor.draft().unwrap().points[1], p(100.0, 60.0));
        canvas.pointer_up_modified(p(210.0, 170.0), false, true);
        assert_eq!(canvas.editor.elements()[0].points[1], p(100.0, 80.0));
        assert!(canvas.editor.undo());
        assert!(canvas.editor.elements().is_empty());
        assert!(!canvas.editor.undo());
        assert!(canvas.editor.redo());
    }
}

#[test]
fn sub_step_shape_does_not_create_degenerate_history() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Rectangle);
    canvas.pointer_down(p(40.0, 40.0));
    canvas.pointer_up_modified(p(43.0, 44.0), true, true);
    assert!(canvas.editor.elements().is_empty());
    assert!(!canvas.editor.undo());
}

#[test]
fn modifier_changes_recompute_from_raw_pointer_without_history() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Rectangle);
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_move(p(57.0, 43.0));
    let revision = canvas.editor.revision();
    assert!(canvas.refresh_constraints(true, false));
    assert_eq!(canvas.editor.draft().unwrap().points[1], p(37.0, 37.0));
    assert!(!canvas.refresh_constraints(true, false));
    assert!(canvas.refresh_constraints(true, true));
    assert_eq!(canvas.editor.draft().unwrap().points[1], p(40.0, 40.0));
    assert!(canvas.refresh_constraints(false, false));
    assert_eq!(canvas.editor.draft().unwrap().points[1], p(37.0, 23.0));
    assert_eq!(canvas.editor.revision(), revision);
    assert!(canvas.editor.elements().is_empty());
    canvas.cancel_gesture();
    assert!(canvas.draft_pointer.is_none());
    assert!(!canvas.refresh_constraints(true, true));
}

#[test]
fn modifier_refresh_does_not_append_freehand_or_reopen_committed_gestures() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Brush);
    canvas.pointer_down(p(30.0, 30.0));
    assert!(!canvas.refresh_constraints(true, true));
    assert_eq!(canvas.editor.draft().unwrap().points.len(), 1);
    canvas.pointer_up(p(40.0, 40.0));
    assert!(canvas.draft_pointer.is_none());
    assert!(!canvas.refresh_constraints(false, false));
    assert!(canvas.editor.draft().is_none());
    assert_eq!(canvas.editor.elements().len(), 1);
}

#[test]
fn configured_angle_snap_reaches_committed_geometry_without_changing_other_tool() {
    let mut canvas = canvas();
    canvas.tool_profiles.toggle_angle_snap(Tool::Line);
    canvas.select_tool(Tool::Line);
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_up(p(70.0, 42.0));
    let stroke = &canvas.editor.elements()[0];
    let dx = stroke.points[1].x - stroke.points[0].x;
    let dy = stroke.points[1].y - stroke.points[0].y;
    assert!((dy.atan2(dx).to_degrees() - 15.0).abs() < 0.001);
    canvas.select_tool(Tool::Arrow);
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_up(p(70.0, 42.0));
    assert_eq!(canvas.editor.elements()[1].points[1], p(50.0, 22.0));
    canvas.select_tool(Tool::Line);
    assert!(canvas.tool_profiles.angle_snap(Tool::Line));
    assert_eq!(canvas.editor.elements().len(), 2);
}

#[test]
fn stroke_coordinates_follow_viewport_and_release_records_last_point() {
    let mut canvas = canvas();
    canvas.viewport.scale = 2.0;
    canvas.select_tool(Tool::Brush);
    assert!(canvas.pointer_down(p(40.0, 60.0)));
    canvas.pointer_move(p(60.0, 60.0));
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.pointer_up(p(80.0, 100.0)));
    assert_eq!(
        canvas.editor.elements()[0].points,
        vec![p(10.0, 20.0), p(20.0, 20.0), p(30.0, 40.0)]
    );
    assert!(canvas.editor.undo());
    assert!(canvas.editor.elements().is_empty());
    assert!(canvas.editor.redo());
}

#[test]
fn cancelled_or_outside_gestures_do_not_commit() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Brush);
    assert!(!canvas.pointer_down(p(0.0, 0.0)));
    canvas.pointer_up(p(30.0, 30.0));
    assert!(canvas.editor.elements().is_empty());
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_move(p(50.0, 50.0));
    assert!(canvas.cancel_gesture());
    canvas.pointer_up(p(60.0, 60.0));
    assert!(canvas.editor.elements().is_empty());
}

#[test]
fn switching_tool_cancels_draft_and_pan_does_not_create_history() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Brush);
    canvas.pointer_down(p(30.0, 30.0));
    assert!(canvas.select_tool(Tool::Pan));
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_up(p(40.0, 50.0));
    assert_eq!(canvas.viewport.origin, p(30.0, 40.0));
    assert!(!canvas.editor.undo());
    assert!(canvas.editor.elements().is_empty());
}

#[test]
fn replacement_resets_document_but_preserves_settings_and_export_snapshot() {
    use crate::stroke_style::{Color, StrokeStyle};
    let mut canvas = canvas();
    canvas.select_tool(Tool::Brush);
    let style = StrokeStyle::new(9.0, Color(12, 34, 56)).unwrap();
    canvas.editor.set_style(style);
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_up(p(40.0, 40.0));
    let snapshot = canvas.editor.export_snapshot();
    let source = canvas.image.as_ref().unwrap().clone();
    let weak = source.pixels.downgrade();
    canvas
        .replace_image(Image {
            width: 16,
            height: 8,
            pixels: vec![0; 16 * 8 * 4].into(),
        })
        .unwrap();
    assert_eq!(canvas.image.as_ref().unwrap().width, 16);
    assert!(!canvas.editor.has_edit_state());
    assert!(!canvas.editor.undo());
    assert!(!canvas.editor.redo());
    assert_eq!(canvas.editor.style(), style);
    assert_eq!(canvas.tool, Tool::Brush);
    assert_eq!(snapshot.elements().len(), 1);
    assert!(
        weak.upgrade().is_some(),
        "export owns its source independently"
    );
    drop(source);
    assert!(
        weak.upgrade().is_some(),
        "editor export snapshot retains committed raster"
    );
    drop(snapshot);
    assert!(
        weak.upgrade().is_none(),
        "canvas no longer retains old source"
    );
}

#[test]
fn tool_switch_restores_profile_without_creating_history_or_mutating_strokes() {
    let mut canvas = canvas();
    canvas.select_tool(Tool::Brush);
    canvas
        .editor
        .set_style(canvas.editor.style().with_width(20.0).unwrap());
    canvas.pointer_down(p(30.0, 30.0));
    canvas.pointer_up(p(40.0, 40.0));
    let revision = canvas.editor.revision();
    canvas.select_tool(Tool::Rectangle);
    assert_eq!(canvas.editor.style().width(), 3.0);
    canvas
        .editor
        .set_style(canvas.editor.style().with_width(6.0).unwrap());
    canvas.select_tool(Tool::Brush);
    assert_eq!(canvas.editor.style().width(), 20.0);
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(canvas.editor.elements()[0].width, 20.0);
    canvas.editor.undo();
    canvas.select_tool(Tool::Rectangle);
    assert_eq!(canvas.editor.style().width(), 6.0);
    assert!(canvas.editor.redo());
}
