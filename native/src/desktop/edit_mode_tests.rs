//! 选择开关保持文档不变；静默只允许继续操作本次新建的标注。
use super::*;
fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}
fn canvas() -> Canvas {
    let mut c = Canvas::new(Some(super::super::image::Image {
        width: 100,
        height: 100,
        pixels: vec![255; 40000].into(),
    }))
    .unwrap();
    // 此夹具直接使用图像坐标，不能沿用普通窗口默认的 20px 视口留边。
    c.viewport.origin = Point::default();
    c.editor.begin(Tool::Rectangle, point(20.0, 20.0)).unwrap();
    c.editor.extend(point(40.0, 40.0)).unwrap();
    assert!(c.editor.commit());
    c.select_tool(Tool::Select);
    c.selected = Some((0, c.editor.revision()));
    c
}

#[test]
fn quiet_mode_cannot_pick_existing_annotations_with_plain_control_alt_or_shift_input() {
    let mut c = canvas();
    c.pinned = true;
    c.selected = None;
    c.selection_enabled = false;
    let stamp = c.editor.document_stamp();
    for (control, alt) in [(false, false), (true, false), (false, true), (true, true)] {
        assert!(c.pointer_down_modified_at(point(20.0, 30.0), 1.0, control, alt));
        assert!(c.selected_indices().is_empty());
        assert!(!c.blocks_wheel());
        c.pointer_up(point(30.0, 30.0));
        assert!(stamp.matches(&c.editor));
    }
    assert!(!c.toggle_selection_at(point(20.0, 30.0)));
    assert!(!c.toggle_selection_at(point(90.0, 90.0)));
    assert!(c.marquee.is_none());
    c.select_tool(Tool::Select);
    assert!(c.selection_enabled);
    assert!(c.pointer_down(point(20.0, 30.0)));
    assert_eq!(c.selected_index(), Some(0));
}
#[test]
fn quiet_creation_selects_only_the_new_annotation_without_reenabling_the_toggle() {
    for tool in [
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Triangle,
        Tool::Polygon,
        Tool::Line,
        Tool::Bezier,
        Tool::Brush,
        Tool::Paintbrush,
        Tool::Mosaic,
        Tool::Blur,
        Tool::Serial,
    ] {
        let mut c = canvas();
        c.pinned = true;
        c.selection_enabled = false;
        c.select_tool(tool);
        if tool == Tool::Blur {
            c.blur_ready = Some((
                c.editor.layers().unwrap().clone(),
                crate::blur_image::BlurImage::with_strength(
                    100,
                    100,
                    c.image.as_ref().unwrap().pixels.clone(),
                    c.tool_profiles.blur_strength(),
                )
                .unwrap(),
                c.editor.annotation_frame(),
                c.editor.raster_base_projection(),
            ));
        }
        assert!(c.pointer_down(point(60.0, 60.0)), "{tool:?}");
        c.pointer_up(point(80.0, 80.0));
        assert_eq!(c.editor.elements().len(), 2, "{tool:?}");
        assert_eq!(c.tool, Tool::Select);
        assert!(!c.selection_enabled);
        assert_eq!(c.selected_index(), Some(1));
        assert!(c.can_select_annotation(1));
        assert!(!c.can_select_annotation(0));
        let stamp = c.editor.document_stamp();
        let rotate = super::super::rotation_handle::position(&c).unwrap();
        assert!(c.pointer_down(rotate));
        assert!(c.rotation_drag.is_some());
        c.cancel_gesture();
        let scale = super::super::scale_handle::position(&c).unwrap();
        assert!(c.pointer_down(scale));
        assert!(c.scale_drag.is_some());
        c.cancel_gesture();
        assert!(stamp.matches(&c.editor));
        assert!(c.pointer_down(point(20.0, 30.0)));
        assert!(c.selected_indices().is_empty());
        assert!(!c.blocks_wheel());
        assert!(c.editor.undo());
        assert_eq!(c.editor.elements().len(), 1);
        assert!(!c.selection_enabled);
    }
}
#[test]
fn quiet_new_annotation_can_move_and_then_loses_its_selection_on_history_change() {
    let mut c = canvas();
    c.pinned = true;
    c.selection_enabled = false;
    c.select_tool(Tool::Rectangle);
    c.pointer_down(point(60.0, 60.0));
    c.pointer_up(point(80.0, 80.0));
    let revision = c.editor.revision();
    assert!(c.pointer_down_modified_at(point(60.0, 70.0), 1.0, false, true));
    assert!(c.pointer_move(point(65.0, 70.0)));
    assert_eq!(c.editor.revision(), revision);
    assert!(c.pointer_up(point(65.0, 70.0)));
    assert_eq!(c.editor.elements()[1].points[0], point(65.0, 60.0));
    assert_eq!(c.selected_index(), Some(1));
    assert!(!c.selection_enabled);
    assert!(c.editor.undo());
    assert!(!c.can_select_annotation(1));
    assert!(super::super::rotation_handle::position(&c).is_none());
}
#[test]
fn pinned_bezier_commit_selects_new_curve_but_ordinary_shape_keeps_its_creation_tool() {
    let mut c = canvas();
    c.pinned = true;
    for tool in [Tool::Rectangle, Tool::Bezier] {
        c.select_tool(tool);
        let count = c.editor.elements().len();
        assert!(c.pointer_down(point(5.0, 5.0)));
        assert!(c.pointer_up(point(15.0, 15.0)));
        assert_eq!(c.editor.elements().len(), count + 1);
        if tool == Tool::Bezier {
            assert_eq!(c.tool, Tool::Select);
            assert_eq!(c.selected_index(), Some(count));
        } else {
            assert_eq!(c.tool, tool);
            assert_eq!(c.selected_index(), None);
        }
    }
}

#[test]
fn quick_rotation_is_one_gesture_and_blank_or_non_pin_never_rotates_the_selection() {
    let mut c = canvas();
    c.pinned = true;
    let revision = c.editor.revision();
    assert!(c.pointer_down_modified_at(point(40.0, 30.0), 1.0, true, false));
    assert!(c.rotation_drag.is_some());
    assert!(c.selection_enabled);
    assert!(c.pointer_move(point(30.0, 40.0)));
    assert_eq!(c.editor.revision(), revision);
    assert!(c.pointer_up(point(30.0, 40.0)));
    assert_eq!(c.editor.revision(), revision + 1);
    assert_eq!(c.selected_index(), Some(0));
    assert!(c.editor.undo());
    assert_eq!(c.editor.elements()[0].rotation, 0.0);
    c.selected = Some((0, c.editor.revision()));
    assert!(c.pointer_down_modified_at(point(95.0, 95.0), 2.0, true, false));
    assert!(c.rotation_drag.is_none());
    assert!(c.selected_indices().is_empty());
    c.pinned = false;
    assert!(c.pointer_down_modified_at(point(40.0, 30.0), 3.0, true, false));
    assert!(c.rotation_drag.is_none());
}

#[test]
fn alt_drag_keeps_multiselection_without_switching_the_selection_toggle() {
    let mut c = canvas();
    c.pinned = true;
    c.editor.begin(Tool::Rectangle, point(60.0, 60.0)).unwrap();
    c.editor.extend(point(80.0, 80.0)).unwrap();
    assert!(c.editor.commit());
    let revision = c.editor.revision();
    c.selected = Some((0, revision));
    c.multi_selection = Some((vec![0, 1], revision));
    assert!(c.pointer_down_modified_at(point(40.0, 30.0), 1.0, false, true));
    assert_eq!(c.group_preview.len(), 2);
    assert!(c.selection_enabled);
    assert!(c.pointer_move(point(45.0, 35.0)));
    assert!(c.pointer_up(point(45.0, 35.0)));
    assert_eq!(c.selected_indices(), vec![0, 1]);
    assert_eq!(c.editor.elements()[0].points[0], point(25.0, 25.0));
    assert_eq!(c.editor.elements()[1].points[0], point(65.0, 65.0));
    assert_eq!(c.editor.revision(), revision + 1);
}
