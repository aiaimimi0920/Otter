//! 文本模态事务回归；只使用合成图片和真实 Canvas，不打开用户输入框。
use super::super::image::Image;
use super::*;
use crate::rotation;
fn image() -> Image {
    Image {
        width: 200,
        height: 200,
        pixels: vec![0; 200 * 200 * 4].into(),
    }
}
fn text(content: &str) -> TextAnnotation {
    TextAnnotation::new(content, "微软雅黑", 16.0, true).unwrap()
}
fn canvas() -> Canvas {
    let mut canvas = Canvas::new(Some(image())).unwrap();
    canvas
        .editor
        .insert_text(
            Point { x: 20.0, y: 30.0 },
            Point { x: 50.0, y: 24.0 },
            text("旧文字\n第二行"),
        )
        .unwrap();
    canvas.editor.rotate(0, 45.0).unwrap();
    canvas.select_tool(Tool::Select);
    canvas.selected = Some((0, canvas.editor.revision()));
    canvas
}
#[test]
fn selected_text_resize_remeasures_layout_and_keeps_center_in_one_transaction() {
    let mut canvas = canvas();
    let before = canvas.editor.elements()[0].clone();
    let next_text = before.text.as_ref().unwrap().with_font_size(20.0).unwrap();
    let size = unsafe { super::super::annotation_text::measure(&next_text) }.unwrap();
    assert!(canvas.resize_selected_axes(Point { x: 2.0, y: 0.5 }));
    let after = canvas.editor.elements()[0].clone();
    assert_eq!(after.text.as_ref(), Some(&next_text));
    assert_eq!(
        rotation::center(&after.points),
        rotation::center(&before.points)
    );
    assert_eq!(after.points[1].x - after.points[0].x, size.x);
    assert_eq!(after.points[1].y - after.points[0].y, size.y);
    assert_eq!(after.rotation, before.rotation);
    assert_eq!(canvas.selected_index(), Some(0));
    assert!(canvas.editor.undo());
    assert_eq!(canvas.editor.elements()[0].points, before.points);
    assert_eq!(canvas.editor.elements()[0].text, before.text);
    assert_eq!(canvas.editor.elements()[0].style(), before.style());
    assert!(canvas.editor.redo());
    assert_eq!(canvas.editor.elements()[0].points, after.points);
    assert_eq!(canvas.editor.elements()[0].text, after.text);
}

#[test]
fn rejected_text_resize_keeps_selection_document_and_history() {
    let mut canvas = canvas();
    let before = canvas.editor.elements()[0].clone();
    let revision = canvas.editor.revision();
    for factors in [
        Point { x: -1.0, y: 4.0 },
        Point { x: 0.0, y: 2.0 },
        Point {
            x: f32::NAN,
            y: 1.0,
        },
        Point {
            x: 100_000.0,
            y: 100_000.0,
        },
    ] {
        assert!(!canvas.resize_selected_axes(factors));
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(canvas.selected_index(), Some(0));
        assert_eq!(canvas.editor.elements()[0].points, before.points);
        assert_eq!(canvas.editor.elements()[0].text, before.text);
    }
    assert!(canvas.editor.undo());
    // 被拒绝的操作未占用历史；一次撤销直接恢复此前旋转。
    assert_eq!(canvas.editor.elements()[0].rotation, 0.0);
}

#[test]
fn restored_text_transform_exceeds_creation_ui_range_and_undo_restores_original() {
    let mut canvas = canvas();
    let before = canvas.editor.elements()[0].clone();
    assert!(canvas.resize_selected_axes(Point {
        x: 1000.0,
        y: 1000.0
    }));
    assert_eq!(
        canvas.editor.elements()[0]
            .text
            .as_ref()
            .unwrap()
            .font_size(),
        16000.0
    );
    assert_eq!(
        rotation::center(&canvas.editor.elements()[0].points),
        rotation::center(&before.points)
    );
    assert!(canvas.editor.undo());
    assert_eq!(canvas.editor.elements()[0].points, before.points);
    assert_eq!(canvas.editor.elements()[0].text, before.text);
    assert_eq!(canvas.editor.elements()[0].style(), before.style());
}

#[test]
fn selected_edit_preserves_style_rotation_world_anchor_and_single_history_step() {
    let mut canvas = canvas();
    let before = canvas.editor.elements()[0].clone();
    let pending = Pending::selected(&canvas).unwrap();
    assert!(pending
        .commit(&mut canvas, text("新文字"), Point { x: 100.0, y: 48.0 })
        .unwrap());
    let after = &canvas.editor.elements()[0];
    assert_eq!(after.style(), before.style());
    assert_eq!(after.rotation, before.rotation);
    let world = |stroke: &crate::editor::Stroke| {
        rotation::rotate(
            stroke.points[0],
            rotation::center(&stroke.points),
            stroke.rotation,
        )
    };
    let a = world(&before);
    let b = world(after);
    assert!((a.x - b.x).hypot(a.y - b.y) < 0.001);
    assert_eq!(canvas.selected_index(), Some(0));
    canvas.editor.undo();
    assert_eq!(canvas.editor.elements()[0].text, before.text);
    assert_eq!(canvas.editor.elements()[0].points, before.points);
    canvas.editor.redo();
    assert_eq!(
        canvas.editor.elements()[0].text.as_ref().unwrap().content(),
        "新文字"
    );
}
#[test]
fn unchanged_content_ignores_remeasurement_and_native_crlf() {
    let mut canvas = canvas();
    let before = canvas.editor.elements()[0].points.clone();
    let pending = Pending::selected(&canvas).unwrap();
    let revision = canvas.editor.revision();
    assert!(!pending
        .commit(
            &mut canvas,
            text("旧文字\r\n第二行"),
            Point { x: 999.0, y: 999.0 }
        )
        .unwrap());
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(canvas.editor.elements()[0].points, before);
}

#[test]
fn modal_result_cannot_edit_replacement_with_identical_revision_and_pixels() {
    let mut canvas = canvas();
    let pending = Pending::selected(&canvas).unwrap();
    canvas.editor = canvas.editor.export_snapshot();
    let revision = canvas.editor.revision();
    let original = canvas.editor.elements()[0].text.clone();
    assert!(!pending
        .commit(&mut canvas, text("迟到文字"), Point { x: 20.0, y: 20.0 })
        .unwrap());
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(canvas.editor.elements()[0].text, original);
}
#[test]
fn pointer_edit_hits_rotated_text_but_not_occluded_text_or_blank_image() {
    let mut canvas = canvas();
    canvas.viewport.origin = Point::default();
    let center = rotation::center(&canvas.editor.elements()[0].points);
    let pending = Pending::at_point(&mut canvas, center).unwrap().unwrap();
    assert_eq!(pending.index, Some(0));
    assert_eq!(pending.text().unwrap().content(), "旧文字\n第二行");
    assert!(Pending::at_point(&mut canvas, Point { x: 180.0, y: 180.0 })
        .unwrap()
        .is_none());
    let style = canvas
        .editor
        .style()
        .with_fill(Some(crate::stroke_style::Color(0, 0, 0)));
    canvas.editor.set_style(style);
    canvas
        .editor
        .begin(Tool::Rectangle, Point { x: 0.0, y: 0.0 })
        .unwrap();
    canvas.editor.extend(Point { x: 150.0, y: 150.0 }).unwrap();
    canvas.editor.commit();
    assert!(Pending::at_point(&mut canvas, center).unwrap().is_none());
    assert_eq!(canvas.editor.elements().len(), 2);
}

#[test]
fn selected_serial_can_edit_label_without_replacing_circle_with_text_bounds() {
    let mut canvas = Canvas::new(Some(image())).unwrap();
    canvas.viewport.origin = Point::default();
    canvas
        .editor
        .insert_serial(Point { x: 60.0, y: 60.0 }, 14.0, "微软雅黑")
        .unwrap();
    canvas.select_tool(Tool::Select);
    let pending = Pending::at_point(&mut canvas, Point { x: 60.0, y: 60.0 })
        .unwrap()
        .unwrap();
    let points = canvas.editor.elements()[0].points.clone();
    assert!(pending
        .commit(&mut canvas, text("A"), Point { x: 100.0, y: 50.0 })
        .unwrap());
    assert_eq!(canvas.editor.elements()[0].points, points);
    assert_eq!(canvas.editor.elements()[0].tool, Tool::Serial);
    canvas.editor.undo();
    assert_eq!(
        canvas.editor.elements()[0].text.as_ref().unwrap().content(),
        "1"
    );
    canvas.select_tool(Tool::Text);
    assert!(Pending::at_point(&mut canvas, Point { x: 60.0, y: 60.0 })
        .unwrap()
        .is_none());
}

#[test]
fn selected_font_change_is_undoable_and_default_font_does_not_mutate_document() {
    let mut canvas = canvas();
    let original = canvas.editor.elements()[0].text.clone().unwrap();
    let revision = canvas.editor.revision();
    canvas.tool_profiles.text_font =
        crate::text_annotation::TextFont::new("Segoe UI", 24.0, false).unwrap();
    assert_eq!(canvas.editor.revision(), revision);
    assert_eq!(canvas.editor.elements()[0].text.as_ref(), Some(&original));
    let pending = Pending::selected(&canvas).unwrap();
    let changed = original.with_font(canvas.tool_profiles.text_font.clone());
    let size = unsafe { super::super::annotation_text::measure(&changed).unwrap() };
    assert!(pending.commit(&mut canvas, changed.clone(), size).unwrap());
    assert_eq!(canvas.editor.elements()[0].text.as_ref(), Some(&changed));
    canvas.editor.undo();
    assert_eq!(canvas.editor.elements()[0].text.as_ref(), Some(&original));
    assert_eq!(canvas.tool_profiles.text_font.font_size(), 24.0);
}

#[test]
fn stale_document_image_or_tool_rejects_modal_result() {
    for case in 0..3 {
        let mut canvas = canvas();
        let pending = Pending::selected(&canvas).unwrap();
        match case {
            0 => {
                canvas
                    .editor
                    .translate(0, Point { x: 1.0, y: 0.0 })
                    .unwrap();
            }
            1 => canvas.image = Some(image()),
            _ => {
                canvas.select_tool(Tool::Brush);
            }
        }
        let revision = canvas.editor.revision();
        assert!(!pending
            .commit(&mut canvas, text("拒绝"), Point { x: 20.0, y: 20.0 })
            .unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(
            canvas.editor.elements()[0].text.as_ref().unwrap().content(),
            "旧文字\n第二行"
        );
    }
}
