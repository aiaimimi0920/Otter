//! 主干的颜色槽按工具族共享；选中对象样式不进入创建默认值，颜色透明度随槽一起恢复。
use crate::{
    editor::Tool,
    stroke_style::{Color, PaintAlpha, StrokeStyle},
};

#[derive(Clone, Copy)]
struct Paint {
    color: Color,
    fill: Option<Color>,
    alpha: PaintAlpha,
}
impl From<StrokeStyle> for Paint {
    fn from(style: StrokeStyle) -> Self {
        Self {
            color: style.color(),
            fill: style.fill(),
            alpha: style.paint_alpha(),
        }
    }
}
pub(super) struct Profiles([Paint; 7]);
impl Default for Profiles {
    fn default() -> Self {
        Self([StrokeStyle::default().into(); 7])
    }
}
fn slot(tool: Tool) -> Option<usize> {
    Some(match tool {
        Tool::Rectangle | Tool::RoundedRectangle => 0,
        Tool::Ellipse => 1,
        Tool::Triangle => 2,
        Tool::Polygon => 3,
        Tool::Line | Tool::Arrow | Tool::Bezier | Tool::Polyline => 4,
        Tool::Brush | Tool::Paintbrush | Tool::Highlighter => 5,
        Tool::Text => 6,
        // 序号已有独立前景/填充配置；导航和像素效果不拥有这些颜色槽。
        _ => return None,
    })
}
impl Profiles {
    pub(super) fn remove(&mut self, color: Color, alpha: u8) {
        for paint in &mut self.0 {
            *paint = super::reset::reset(
                StrokeStyle::default()
                    .with_color(paint.color)
                    .with_fill(paint.fill)
                    .with_paint_alpha(paint.alpha),
                false,
                color,
                alpha,
            )
            .into();
        }
    }
    pub(super) fn switch(
        &mut self,
        previous: Tool,
        next: Tool,
        current: StrokeStyle,
        properties: StrokeStyle,
    ) -> StrokeStyle {
        if previous == next {
            return properties;
        }
        if let Some(slot) = slot(previous) {
            self.0[slot] = current.into();
        }
        let Some(slot) = slot(next) else {
            return properties;
        };
        let paint = self.0[slot];
        properties
            .with_color(paint.color)
            .with_fill(paint.fill)
            .with_paint_alpha(paint.alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_profiles::ToolProfiles;
    #[test]
    fn families_restore_their_own_color_fill_and_alpha_through_navigation_and_serial() {
        let mut p = ToolProfiles::default();
        let rectangle = StrokeStyle::default()
            .with_color(Color(1, 2, 3))
            .with_fill(Some(Color(4, 5, 6)))
            .with_paint_alpha(PaintAlpha::new(0.3, 0.6).unwrap());
        let ellipse = p.switch(Tool::Rectangle, Tool::Ellipse, rectangle);
        assert_eq!(ellipse.color(), StrokeStyle::default().color());
        assert_eq!(ellipse.fill(), None);
        let text = p
            .switch(Tool::Ellipse, Tool::Text, ellipse)
            .with_color(Color(7, 8, 9));
        let select = p.switch(Tool::Text, Tool::Select, text);
        let pan = p.switch(Tool::Select, Tool::Pan, select);
        let restored = p.switch(Tool::Pan, Tool::RoundedRectangle, pan);
        assert_eq!(restored.color(), rectangle.color());
        assert_eq!(restored.fill(), rectangle.fill());
        assert_eq!(restored.paint_alpha(), rectangle.paint_alpha());
        let serial = p.switch(Tool::RoundedRectangle, Tool::Serial, restored);
        assert_eq!(serial.fill(), Some(Color(0, 0, 0)));
        assert_eq!(
            p.switch(Tool::Serial, Tool::Text, serial).color(),
            text.color()
        );
    }
    #[test]
    fn line_and_brush_variants_share_only_their_familys_colors_not_width_or_opacity() {
        let mut p = ToolProfiles::default();
        let line = StrokeStyle::default()
            .with_color(Color(2, 3, 4))
            .with_width(7.0)
            .unwrap();
        let curve = p.switch(Tool::Line, Tool::Bezier, line);
        assert_eq!(curve.color(), line.color());
        assert_eq!(curve.width(), 3.0);
        let brush = p
            .switch(Tool::Bezier, Tool::Brush, curve)
            .with_color(Color(9, 8, 7))
            .with_opacity(0.4)
            .unwrap();
        let paintbrush = p.switch(Tool::Brush, Tool::Paintbrush, brush);
        assert_eq!(paintbrush.color(), brush.color());
        assert_eq!(paintbrush.width(), 16.0);
        assert_eq!(paintbrush.opacity(), 1.0);
        assert_eq!(
            p.switch(Tool::Paintbrush, Tool::Line, paintbrush).color(),
            line.color()
        );
    }
}
