//! 草稿提交的唯一事务；平滑只物化一次，普通画笔荧光保留画笔采样来源。
use super::*;

impl Editor {
    pub fn commit(&mut self) -> bool {
        self.commit_smoothed(0.0)
    }

    pub fn commit_smoothed(&mut self, smoothing: f32) -> bool {
        self.draft_motion = None;
        let brush_highlighter = std::mem::take(&mut self.draft_brush_highlighter);
        let Some(mut stroke) = self.draft.take() else {
            return false;
        };
        // 单击画笔保留一个点；零长度线和零面积形状不制造历史。
        if stroke.tool == Tool::Polyline && stroke.points.len() < 2 {
            return false;
        }
        if !matches!(
            stroke.tool,
            Tool::Brush
                | Tool::Highlighter
                | Tool::Polyline
                | Tool::Paintbrush
                | Tool::Mosaic
                | Tool::Blur
        ) && (stroke.points.len() < 2 || stroke.points.first() == stroke.points.last())
        {
            return false;
        }
        if matches!(
            stroke.tool,
            Tool::Rectangle
                | Tool::RoundedRectangle
                | Tool::Ellipse
                | Tool::Triangle
                | Tool::Polygon
        ) && (stroke.points[0].x == stroke.points[1].x
            || stroke.points[0].y == stroke.points[1].y)
        {
            return false;
        }
        if brush_highlighter || matches!(stroke.tool, Tool::Brush | Tool::Paintbrush) {
            crate::freehand::smooth(&mut stroke.points, smoothing);
        }
        self.retained_points += stroke.points.len();
        let index = self.elements.len();
        self.elements.push(stroke);
        self.retained_points -= self.history.record(Entry::added(index));
        self.revision = self.revision.wrapping_add(1);
        true
    }
}
