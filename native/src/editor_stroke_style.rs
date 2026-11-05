//! 活动 Stroke 与下一次手势样式的完整桥；历史恢复不能遗漏独立颜色 alpha。
use super::{Stroke, StrokeStyle, Tool};
impl Stroke {
    pub fn has_straight_arrow(&self) -> bool {
        self.tool == Tool::Arrow || (self.tool == Tool::Line && self.arrow)
    }
    pub fn style(&self) -> StrokeStyle {
        StrokeStyle::restored(self.width, self.color)
            .unwrap()
            .with_opacity(self.opacity)
            .unwrap()
            .with_fill(self.fill)
            .with_corner_radius_value(self.corner_radius)
            .unwrap()
            .with_dash(self.dash)
            .with_sides_value(self.sides)
            .unwrap()
            .with_arrow(self.arrow)
            .with_mosaic_cell_value(self.mosaic_cell)
            .unwrap()
            .with_paint_alpha(self.paint_alpha)
    }
    pub(crate) fn apply_style(&mut self, style: StrokeStyle) -> StrokeStyle {
        let previous = self.style();
        self.width = style.width();
        self.color = style.color();
        self.opacity = style.opacity();
        self.fill = style.fill();
        self.paint_alpha = style.paint_alpha();
        self.corner_radius = style.corner_radius();
        self.dash = style.dash();
        self.sides = style.sides();
        self.arrow = style.arrow();
        self.mosaic_cell = style.mosaic_cell();
        previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::Editor,
        stroke_style::{Color, PaintAlpha},
        viewport::Point,
    };
    #[test]
    fn alpha_follows_style_history_snapshots_duplication_and_adoption() {
        let alpha = PaintAlpha::new(0.25, 0.625).unwrap();
        let style = StrokeStyle::default()
            .with_fill(Some(Color(0, 0, 255)))
            .with_opacity(0.5)
            .unwrap()
            .with_paint_alpha(alpha);
        let mut editor = Editor::default();
        editor.set_style(style);
        editor
            .begin(Tool::Rectangle, Point { x: 4.0, y: 4.0 })
            .unwrap();
        editor.extend(Point { x: 40.0, y: 40.0 }).unwrap();
        assert!(editor.commit());
        let id = editor.elements()[0].annotation_id().to_owned();
        assert_eq!(editor.elements()[0].style(), style);
        let snapshot = editor.export_snapshot();
        let changed = style.with_width(8.0).unwrap().with_opacity(0.75).unwrap();
        assert!(editor.restyle(0, changed));
        assert_eq!(editor.elements()[0].paint_alpha, alpha);
        assert!(editor.undo());
        assert_eq!(editor.elements()[0].style(), style);
        assert!(editor.redo());
        assert_eq!(editor.elements()[0].style(), changed);
        editor.duplicate(0).unwrap();
        assert_eq!(editor.elements()[1].paint_alpha, alpha);
        assert_ne!(editor.elements()[1].annotation_id(), id);
        assert_eq!(snapshot.elements()[0].style(), style);
        let mut target = Editor::default();
        assert!(target.adopt_snapshot(0, &snapshot).unwrap());
        assert_eq!(target.elements()[0].style(), style);
        assert_eq!(target.elements()[0].annotation_id(), id);
        assert!(target.undo());
        assert!(target.elements().is_empty());
        assert!(target.redo());
        assert_eq!(target.elements()[0].paint_alpha, alpha);
    }
    #[test]
    fn explicit_rgb_replacement_resets_only_its_own_alpha() {
        let style = StrokeStyle::default().with_paint_alpha(PaintAlpha::new(0.2, 0.7).unwrap());
        let outline = style.with_color(Color(1, 2, 3));
        assert_eq!(outline.paint_alpha(), PaintAlpha::new(1.0, 0.7).unwrap());
        let fill = style.with_fill(Some(Color(4, 5, 6)));
        assert_eq!(fill.paint_alpha(), PaintAlpha::new(0.2, 1.0).unwrap());
        assert_eq!(style.with_fill(None).paint_alpha().stroke(), 0.2);
    }
}
