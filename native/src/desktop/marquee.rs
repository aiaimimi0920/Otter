//! 标注框选只保存两个图像坐标与文档身份；松手计算选择，不创建编辑历史。
use super::canvas::Canvas;
use crate::{
    editor::{DocumentStamp, Editor},
    viewport::{Point, Viewport},
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

pub(super) struct Marquee {
    start: Point,
    end: Point,
    stamp: DocumentStamp,
    initial: Vec<usize>,
}
impl Marquee {
    pub(super) fn new(start: Point, editor: &Editor) -> Self {
        Self {
            start,
            end: start,
            stamp: editor.document_stamp(),
            initial: Vec::new(),
        }
    }
    fn bounds(&self) -> (Point, Point) {
        (
            Point {
                x: self.start.x.min(self.end.x),
                y: self.start.y.min(self.end.y),
            },
            Point {
                x: self.start.x.max(self.end.x),
                y: self.start.y.max(self.end.y),
            },
        )
    }
}
impl Canvas {
    pub(super) fn escape_selection(&mut self) -> bool {
        if self.marquee.is_some() {
            return self.cancel_gesture();
        }
        let cleared = self.selected.take().is_some();
        self.multi_selection = None;
        self.cancel_gesture() || cleared
    }
    pub(super) fn begin_additive_marquee(&mut self, point: Point) -> bool {
        let mut drag = Marquee::new(point, &self.editor);
        drag.initial = self.selected_indices();
        self.marquee = Some(drag);
        true
    }
    pub(super) fn move_marquee(&mut self, screen: Point) -> bool {
        let Some(drag) = self.marquee.as_mut() else {
            return false;
        };
        if !drag.stamp.matches(&self.editor) {
            self.marquee = None;
            return true;
        }
        let point = self.viewport.image_point(screen);
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x.abs() > 1_000_000.0
            || point.y.abs() > 1_000_000.0
        {
            return false;
        }
        let changed = point != drag.end;
        drag.end = point;
        changed
    }
    pub(super) fn finish_marquee(&mut self) {
        let Some(drag) = self.marquee.take() else {
            return;
        };
        if !drag.stamp.matches(&self.editor) {
            return;
        }
        let (low, high) = drag.bounds();
        // 点击空白只清空选择；完整包含视觉边界才入选，避免掠过边缘误选。
        if (high.x - low.x) * self.viewport.scale < 3.0
            || (high.y - low.y) * self.viewport.scale < 3.0
        {
            return;
        }
        let mut indices: Vec<_> = self
            .editor
            .elements()
            .iter()
            .enumerate()
            .filter_map(|(index, stroke)| {
                let bounds = crate::group_bounds::bounds(stroke)?;
                (bounds.low.x >= low.x
                    && bounds.low.y >= low.y
                    && bounds.high.x <= high.x
                    && bounds.high.y <= high.y)
                    .then_some(index)
            })
            .collect();
        indices.extend(drag.initial);
        indices.sort_unstable();
        indices.dedup();
        let revision = self.editor.revision();
        self.selected = indices.first().map(|index| (*index, revision));
        self.multi_selection = (!indices.is_empty()).then_some((indices, revision));
    }
}

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    drag: Option<&Marquee>,
    viewport: Viewport,
) {
    let Some(drag) = drag else {
        return;
    };
    let (low, high) = drag.bounds();
    let rect = D2D_RECT_F {
        left: viewport.origin.x + low.x * viewport.scale,
        top: viewport.origin.y + low.y * viewport.scale,
        right: viewport.origin.x + high.x * viewport.scale,
        bottom: viewport.origin.y + high.y * viewport.scale,
    };
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.1,
        g: 0.8,
        b: 1.0,
        a: 0.12,
    });
    target.FillRectangle(&rect, brush);
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.1,
        g: 0.8,
        b: 1.0,
        a: 1.0,
    });
    target.DrawRectangle(&rect, brush, 1.0, None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Tool;
    fn canvas() -> Canvas {
        let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
            width: 100,
            height: 100,
            pixels: vec![255; 40000].into(),
        }))
        .unwrap();
        canvas.viewport = Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        for x in [20.0, 50.0, 80.0] {
            canvas
                .editor
                .begin(Tool::Brush, Point { x, y: 30.0 })
                .unwrap();
            canvas.editor.commit();
        }
        canvas.select_tool(Tool::Select);
        canvas
    }
    #[test]
    fn shift_marquee_adds_deduplicated_indices_and_preserves_selection_on_cancel() {
        let mut canvas = canvas();
        assert!(canvas.toggle_selection_at(Point { x: 20.0, y: 30.0 }));
        assert_eq!(canvas.selected_indices(), [0]);
        let start = Point { x: 10.0, y: 10.0 };
        let end = Point { x: 60.0, y: 50.0 };
        assert!(canvas.toggle_selection_at(start));
        assert!(canvas.blocks_wheel());
        canvas.pointer_move(end);
        assert_eq!(canvas.selected_indices(), [0]);
        assert!(canvas.escape_selection());
        assert!(!canvas.blocks_wheel());
        assert_eq!(canvas.selected_indices(), [0]);
        let revision = canvas.editor.revision();
        canvas.toggle_selection_at(start);
        canvas.pointer_up(end);
        assert_eq!(canvas.selected_indices(), [0, 1]);
        assert_eq!(canvas.editor.revision(), revision);
        // Shift 点击仍切换命中对象，不启动框选。
        canvas.toggle_selection_at(Point { x: 20.0, y: 30.0 });
        assert_eq!(canvas.selected_indices(), [1]);
        assert!(canvas.marquee.is_none());
    }
    #[test]
    fn shift_marquee_empty_click_and_stale_revision_do_not_apply_initial_indices() {
        let mut canvas = canvas();
        canvas.toggle_selection_at(Point { x: 50.0, y: 30.0 });
        let start = Point { x: 10.0, y: 10.0 };
        canvas.toggle_selection_at(start);
        canvas.pointer_up(start);
        assert_eq!(canvas.selected_indices(), [1]);
        canvas.toggle_selection_at(start);
        canvas.editor.remove(0);
        canvas.pointer_up(Point { x: 90.0, y: 50.0 });
        assert!(canvas.selected_indices().is_empty());
        assert!(canvas.marquee.is_none());
    }
    #[test]
    fn forward_reverse_marquee_selects_group_without_edit_and_group_delete_undoes_once() {
        for reverse in [false, true] {
            let mut canvas = canvas();
            let (a, b) = (Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 50.0 });
            let (a, b) = if reverse { (b, a) } else { (a, b) };
            let revision = canvas.editor.revision();
            assert!(canvas.pointer_down(a));
            assert!(canvas.marquee.is_some());
            assert!(canvas.pointer_move(b));
            assert!(canvas.selected_indices().is_empty());
            assert!(canvas.pointer_up(b));
            assert_eq!(canvas.selected_indices(), [0, 1]);
            assert_eq!(canvas.editor.revision(), revision);
            assert!(canvas.delete_target());
            assert_eq!(canvas.editor.elements().len(), 1);
            assert!(canvas.editor.undo());
            assert_eq!(canvas.editor.elements().len(), 3);
        }
    }
    #[test]
    fn cancellation_and_document_replacement_discard_marquee() {
        let mut canvas = canvas();
        let start = Point { x: 10.0, y: 10.0 };
        let end = Point { x: 90.0, y: 50.0 };
        canvas.pointer_down(start);
        canvas.pointer_move(end);
        assert!(canvas.cancel_gesture());
        assert!(canvas.marquee.is_none());
        assert!(canvas.selected_indices().is_empty());
        canvas.pointer_down(start);
        canvas.editor = canvas.editor.export_snapshot();
        canvas.pointer_up(end);
        assert!(canvas.selected_indices().is_empty());
        assert!(canvas.marquee.is_none());
    }
    #[test]
    fn scale_mapping_click_threshold_and_nonfinite_pointer_are_safe() {
        let mut canvas = canvas();
        canvas.viewport = Viewport {
            origin: Point { x: 100.0, y: 100.0 },
            scale: 2.0,
        };
        let start = Point { x: 120.0, y: 120.0 };
        canvas.pointer_down(start);
        assert!(!canvas.pointer_move(Point {
            x: f32::NAN,
            y: 10.0
        }));
        canvas.pointer_up(Point { x: 220.0, y: 200.0 });
        assert_eq!(canvas.selected_indices(), [0, 1]);
        canvas.pointer_down(start);
        canvas.pointer_up(Point { x: 121.0, y: 121.0 });
        assert!(canvas.selected_indices().is_empty());
    }
}
