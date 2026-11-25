//! 多选绑定同一文档版本；主锚点改变或历史切换即失效，不让旧索引作用于新对象。
use super::canvas::Canvas;
use crate::{
    editor::{DocumentStamp, TranslationBounds},
    viewport::Point,
};

pub(super) struct GroupTranslation {
    stamp: DocumentStamp,
    bounds: TranslationBounds,
}

impl Canvas {
    pub(super) fn begin_group_drag(&mut self, screen: Point) -> bool {
        let Some(indices) = self.selected_group().filter(|indices| indices.len() > 1) else {
            return false;
        };
        let Some(point) = self.image_point(screen, false) else {
            return false;
        };
        let hit = unsafe {
            super::selection_hit::pick(
                &self.factory,
                &self.editor,
                point,
                5.0 / self.viewport.scale,
            )
        };
        if !matches!(hit, Ok(Some(index)) if indices.binary_search(&index).is_ok()) {
            return false;
        }
        self.begin_group_translation(point)
    }

    pub(super) fn begin_group_translation(&mut self, point: Point) -> bool {
        let Some(indices) = self.selected_group().filter(|indices| indices.len() > 1) else {
            return false;
        };
        let originals = indices
            .iter()
            .map(|&index| (index, &self.editor.elements()[index]));
        let Some(bounds) = TranslationBounds::new(
            originals
                .clone()
                .flat_map(|(_, stroke)| stroke.points.iter().copied()),
        ) else {
            return false;
        };
        // One preview allocation per gesture; every sample starts from immutable original geometry.
        self.group_preview = originals
            .map(|(index, stroke)| (index, stroke.clone()))
            .collect();
        self.scale_display_revision = None;
        self.group_translation = Some(GroupTranslation {
            stamp: self.editor.document_stamp(),
            bounds,
        });
        self.selection_drag = Some((point, Point::default()));
        true
    }

    // Selection can change without an edit revision. Never reuse cached bounds for a new group.
    pub(super) fn discard_stale_group_translation(&mut self) -> bool {
        if self.group_translation.as_ref().is_some_and(|drag| {
            !drag.stamp.matches(&self.editor)
                || !self.selected_group().is_some_and(|indices| {
                    indices
                        .iter()
                        .copied()
                        .eq(self.group_preview.iter().map(|(index, _)| *index))
                })
        }) {
            self.cancel_gesture();
            return true;
        }
        false
    }

    pub(super) fn move_group_drag(&mut self, screen: Point, start: Point, previous: Point) -> bool {
        if self.discard_stale_group_translation() {
            return true;
        }
        let point = self.viewport.image_point(screen);
        let offset = self.move_axis.constrain(Point {
            x: point.x - start.x,
            y: point.y - start.y,
        });
        let drag = self
            .group_translation
            .as_ref()
            .expect("active group translation");
        if offset == previous || !drag.bounds.permits(offset) {
            return false;
        }
        for (index, preview) in &mut self.group_preview {
            preview.apply_valid_translation_preview(&self.editor.elements()[*index], offset);
        }
        self.selection_drag = Some((start, offset));
        true
    }
    pub(super) fn commit_group_drag(&mut self, offset: Point) {
        self.group_translation = None;
        self.group_preview = Vec::new();
        let indices = self.selected_indices();
        match self.editor.translate_many(&indices, offset) {
            Ok(true) => {
                let revision = self.editor.revision();
                self.selected = indices.first().map(|index| (*index, revision));
                self.multi_selection = Some((indices, revision));
            }
            Ok(false) => {}
            Err(error) => eprintln!("native-group-move: {error:?}; preview discarded"),
        }
    }
    pub(super) fn selected_group(&self) -> Option<&[usize]> {
        let (indices, revision) = self.multi_selection.as_ref()?;
        if *revision != self.editor.revision()
            || indices.is_empty()
            || self.selected != Some((indices[0], *revision))
            || indices
                .last()
                .is_some_and(|index| *index >= self.editor.elements().len())
        {
            return None;
        }
        Some(indices)
    }
    pub(super) fn selected_indices_ref(&self) -> &[usize] {
        if let Some(indices) = self.selected_group() {
            return indices;
        }
        self.selected
            .as_ref()
            .filter(|(index, revision)| {
                *revision == self.editor.revision() && *index < self.editor.elements().len()
            })
            .map_or(&[], |(index, _)| std::slice::from_ref(index))
    }
    pub(super) fn selected_indices(&self) -> Vec<usize> {
        self.selected_indices_ref().to_vec()
    }
    pub(super) fn toggle_selection_at(&mut self, point: Point) -> bool {
        if !self.selection_enabled {
            return false;
        }
        self.cancel_gesture();
        let Some(point) = self.image_point(point, false) else {
            return false;
        };
        let index = unsafe {
            super::selection_hit::pick(
                &self.factory,
                &self.editor,
                point,
                5.0 / self.viewport.scale,
            )
        };
        let index = match index {
            Ok(Some(index)) => index,
            Ok(None) => return self.begin_additive_marquee(point),
            Err(error) => {
                eprintln!("native-multiselect: {error}");
                return false;
            }
        };
        let mut indices = self.selected_indices();
        match indices.binary_search(&index) {
            Ok(position) => {
                indices.remove(position);
            }
            Err(position) => indices.insert(position, index),
        }
        let revision = self.editor.revision();
        self.selected = indices.first().map(|index| (*index, revision));
        self.multi_selection = (!indices.is_empty()).then_some((indices, revision));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::Tool, viewport::Viewport};
    fn canvas() -> Canvas {
        let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
            width: 64,
            height: 32,
            pixels: vec![255; 64 * 32 * 4].into(),
        }))
        .unwrap();
        canvas.viewport = Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        for x in [8.0, 32.0, 56.0] {
            canvas
                .editor
                .begin(Tool::Brush, Point { x, y: 16.0 })
                .unwrap();
            assert!(canvas.editor.commit());
        }
        canvas.select_tool(Tool::Select);
        canvas
    }
    #[test]
    fn apply_style_updates_whole_selection_and_preserves_group_for_next_command() {
        let mut canvas = canvas();
        for x in [8.0, 56.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        let old = canvas.editor.elements()[0].style();
        let style = old
            .with_color(crate::stroke_style::Color(10, 30, 80))
            .with_width(7.0)
            .unwrap();
        canvas.editor.set_style(style);
        let revision = canvas.editor.revision();
        assert!(canvas.restyle_target());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(canvas.selected_indices(), vec![0, 2]);
        assert_eq!(canvas.editor.elements()[0].style(), style);
        assert_eq!(canvas.editor.elements()[1].style(), old);
        assert_eq!(canvas.editor.elements()[2].style(), style);
        assert!(!canvas.restyle_target());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert!(canvas.editor.undo());
        assert!(canvas.editor.elements().iter().all(|s| s.style() == old));
        assert!(!canvas.restyle_target()); // 旧版本选择不作用于撤销后的文档。
        assert!(canvas.editor.redo());
        assert_eq!(canvas.editor.elements()[2].style(), style);
    }

    #[test]
    fn toggle_noncontiguous_selection_delete_and_undo_as_one_action() {
        let mut canvas = canvas();
        assert!(canvas.toggle_selection_at(Point { x: 56.0, y: 16.0 }));
        assert!(canvas.toggle_selection_at(Point { x: 8.0, y: 16.0 }));
        assert_eq!(canvas.selected_indices(), vec![0, 2]);
        assert_eq!(canvas.selected_index(), None);
        assert!(!canvas.duplicate_selected());
        assert_eq!(canvas.selected_indices(), vec![0, 2]);
        assert!(canvas.delete_target());
        assert_eq!(canvas.editor.elements().len(), 1);
        assert_eq!(canvas.editor.elements()[0].points[0].x, 32.0);
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements().len(), 3);
        assert!(canvas.selected_indices().is_empty());
    }
    #[test]
    fn toggle_off_restore_single_and_stale_group_cannot_delete_new_indices() {
        let mut canvas = canvas();
        for x in [8.0, 56.0, 8.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        assert_eq!(canvas.selected_index(), Some(2));
        assert!(canvas.toggle_selection_at(Point { x: 8.0, y: 16.0 }));
        assert!(canvas.editor.remove(0));
        assert!(canvas.selected_indices().is_empty());
        assert!(!canvas.delete_target());
        assert_eq!(canvas.editor.elements().len(), 2);
        assert!(canvas.toggle_selection_at(Point { x: 32.0, y: 16.0 }));
        assert_eq!(canvas.selected_indices(), vec![0]);
        canvas.select_tool(Tool::Brush);
        assert!(canvas.selected_indices().is_empty());
    }
    #[test]
    fn group_drag_cancel_stale_and_invalid_move_never_modify_document() {
        let mut canvas = canvas();
        for x in [8.0, 56.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        let start = Point { x: 8.0, y: 16.0 };
        let revision = canvas.editor.revision();
        assert!(canvas.pointer_down(start));
        assert!(canvas.pointer_move(Point { x: 10.0, y: 20.0 }));
        let points: Vec<_> = canvas
            .group_preview
            .iter()
            .map(|(_, s)| s.points.clone())
            .collect();
        for x in [f32::NAN, f32::INFINITY, 999_980.0] {
            assert!(!canvas.pointer_move(Point { x, y: 16.0 }));
            assert_eq!(
                canvas
                    .group_preview
                    .iter()
                    .map(|(_, s)| s.points.clone())
                    .collect::<Vec<_>>(),
                points
            );
        }
        assert!(canvas.cancel_gesture());
        assert!(canvas.group_preview.is_empty());
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.pointer_down(start));
        assert!(canvas.editor.undo());
        assert!(canvas.pointer_move(start));
        assert!(canvas.group_preview.is_empty());
        assert!(canvas.selection_drag.is_none());
    }
    #[test]
    fn reselection_without_revision_change_does_not_commit_an_old_group_drag() {
        let mut canvas = canvas();
        for x in [8.0, 56.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        let start = Point { x: 8.0, y: 16.0 };
        let end = Point { x: 10.0, y: 20.0 };
        let revision = canvas.editor.revision();
        assert!(canvas.pointer_down(start));
        assert!(canvas.pointer_move(end));
        assert!(!canvas.pointer_move(end));
        canvas.multi_selection = Some((vec![0, 1], revision));
        assert!(canvas.pointer_up(end));
        assert!(canvas.group_translation.is_none());
        assert!(canvas.group_preview.is_empty());
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(canvas.editor.elements()[0].points[0], start);
        assert_eq!(
            canvas.editor.elements()[1].points[0],
            Point { x: 32.0, y: 16.0 }
        );
        assert_eq!(canvas.selected_indices(), vec![0, 1]);
    }

    #[test]
    fn rotate_command_keeps_group_selected_and_undoes_as_one_transaction() {
        let mut canvas = canvas();
        for x in [8.0, 56.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        let revision = canvas.editor.revision();
        assert!(canvas.rotate_selected(90.0));
        assert_eq!(canvas.selected_indices(), vec![0, 2]);
        assert_eq!(canvas.editor.revision(), revision + 1);
        let first = canvas.editor.elements()[0].points[0];
        assert!((first.x - 32.0).abs() < 0.001);
        assert!((first.y + 8.0).abs() < 0.001);
        assert_eq!(
            canvas.editor.elements()[1].points[0],
            Point { x: 32.0, y: 16.0 }
        );
        assert!(canvas.editor.undo());
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: 8.0, y: 16.0 }
        );
        assert_eq!(
            canvas.editor.elements()[2].points[0],
            Point { x: 56.0, y: 16.0 }
        );
    }
    #[test]
    fn group_resize_uses_directwrite_for_text_and_retains_selection() {
        let mut canvas = canvas();
        let text =
            crate::text_annotation::TextAnnotation::new("组缩放 Test", "微软雅黑", 16.0, false)
                .unwrap();
        let size = unsafe { super::super::annotation_text::measure(&text) }.unwrap();
        canvas
            .editor
            .insert_text(Point { x: 4.0, y: 4.0 }, size, text.clone())
            .unwrap();
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 3], revision));
        assert!(canvas.resize_selected(1.5));
        assert_eq!(canvas.selected_indices(), vec![0, 3]);
        assert_eq!(canvas.editor.revision(), revision + 1);
        let measured =
            unsafe { super::super::annotation_text::measure(&text.with_font_size(24.0).unwrap()) }
                .unwrap();
        let points = &canvas.editor.elements()[3].points;
        assert!((points[1].x - points[0].x - measured.x).abs() < 0.001);
        assert!((points[1].y - points[0].y - measured.y).abs() < 0.001);
        assert!(canvas.editor.undo());
        assert_eq!(
            canvas.editor.elements()[3]
                .text
                .as_ref()
                .unwrap()
                .font_size(),
            16.0
        );
    }
    #[test]
    fn group_axis_resize_scales_positions_and_preserves_unselected_object() {
        let mut canvas = canvas();
        for x in [8.0, 56.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        let revision = canvas.editor.revision();
        assert!(canvas.resize_selected_axes(Point { x: 2.0, y: 1.0 }));
        assert_eq!(canvas.selected_indices(), vec![0, 2]);
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: -16.0, y: 16.0 }
        );
        assert_eq!(
            canvas.editor.elements()[1].points[0],
            Point { x: 32.0, y: 16.0 }
        );
        assert_eq!(
            canvas.editor.elements()[2].points[0],
            Point { x: 80.0, y: 16.0 }
        );
        assert!(canvas.editor.undo());
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: 8.0, y: 16.0 }
        );
    }
    #[test]
    fn own_center_mode_preserves_positions_for_rotation_and_axis_scaling() {
        let mut canvas = canvas();
        for x in [8.0, 56.0] {
            assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
        }
        canvas.transform_own = true;
        let width = canvas.editor.elements()[0].width;
        assert!(canvas.rotate_selected(90.0));
        assert!(canvas.resize_selected_axes(Point { x: 2.0, y: 1.0 }));
        assert_eq!(canvas.selected_indices(), vec![0, 2]);
        assert_eq!(
            canvas.editor.elements()[0].points[0],
            Point { x: 8.0, y: 16.0 }
        );
        assert_eq!(
            canvas.editor.elements()[2].points[0],
            Point { x: 56.0, y: 16.0 }
        );
        assert_eq!(canvas.editor.elements()[0].width, width * 1.5);
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements()[0].width, width);
        assert_eq!(canvas.editor.elements()[0].rotation, 90.0);
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements()[0].rotation, 0.0);
    }
    #[test]
    fn axis_constrained_single_and_group_drag_share_preview_and_commit_offsets() {
        use super::super::transform_mode::MoveAxis;
        for group in [false, true] {
            for (axis, expected) in [
                (MoveAxis::X, Point { x: 4.0, y: 0.0 }),
                (MoveAxis::Y, Point { x: 0.0, y: 6.0 }),
                (MoveAxis::Free, Point { x: 4.0, y: 6.0 }),
            ] {
                let mut canvas = canvas();
                canvas.move_axis = axis;
                if group {
                    for x in [8.0, 56.0] {
                        assert!(canvas.toggle_selection_at(Point { x, y: 16.0 }));
                    }
                }
                let revision = canvas.editor.revision();
                assert!(canvas.pointer_down(Point { x: 8.0, y: 16.0 }));
                assert!(canvas.pointer_move(Point { x: 12.0, y: 22.0 }));
                assert_eq!(canvas.selection_drag.unwrap().1, expected);
                if group {
                    assert_eq!(
                        canvas.group_preview[0].1.points[0],
                        Point {
                            x: 8.0 + expected.x,
                            y: 16.0 + expected.y
                        }
                    );
                }
                assert!(canvas.pointer_up(Point { x: 12.0, y: 22.0 }));
                assert_eq!(canvas.editor.revision(), revision + 1);
                assert_eq!(
                    canvas.editor.elements()[0].points[0],
                    Point {
                        x: 8.0 + expected.x,
                        y: 16.0 + expected.y
                    }
                );
                assert_eq!(
                    canvas.editor.elements()[2].points[0],
                    Point {
                        x: 56.0 + if group { expected.x } else { 0.0 },
                        y: 16.0 + if group { expected.y } else { 0.0 }
                    }
                );
                assert!(canvas.editor.undo());
                assert_eq!(
                    canvas.editor.elements()[0].points[0],
                    Point { x: 8.0, y: 16.0 }
                );
            }
        }
    }
}
