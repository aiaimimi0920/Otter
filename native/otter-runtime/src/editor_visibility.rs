//! Shared visible-pen checks for drawing, hit testing, and window regions; zero width must not rely on D2D's default hairline behavior.
use super::{Stroke, Tool};
impl Stroke {
    pub fn visible_outline(&self) -> bool {
        self.width > 0.0 && self.paint_alpha.stroke() > 0.0
    }
    pub fn visible_fill(&self) -> bool {
        self.fill.is_some() && self.paint_alpha.fill() > 0.0
    }
    pub fn visible_paint(&self) -> bool {
        if self.tool.spotlight() || matches!(self.tool, Tool::Mosaic | Tool::Blur | Tool::Magnifier)
        {
            return true;
        }
        if self.opacity <= 0.0 {
            return false;
        }
        let text = self.paint_alpha.stroke() > 0.0
            && self
                .text
                .as_ref()
                .is_some_and(|t| t.content().chars().any(|c| !c.is_whitespace() && c != '\0'));
        match self.tool {
            Tool::Text => text || self.visible_fill(),
            Tool::Rectangle
            | Tool::RoundedRectangle
            | Tool::Ellipse
            | Tool::Triangle
            | Tool::Polygon
            | Tool::Serial => {
                let Some((a, b)) = self.points.first().zip(self.points.last()) else {
                    return false;
                };
                a.x != b.x
                    && a.y != b.y
                    && (self.visible_outline()
                        || self.visible_fill()
                        || (self.tool == Tool::Serial && text))
            }
            _ => self.visible_outline(),
        }
    }
}
