//! 效果移动预览复用一次克隆的点缓冲，以原始几何计算，避免累计浮点漂移。
use super::*;
impl Stroke {
    pub(crate) fn update_translation_preview(
        &mut self,
        original: &Self,
        offset: Point,
    ) -> Result<(), EditError> {
        validate(offset)?;
        if self.points.len() != original.points.len() || self.tool != original.tool {
            return Err(EditError::InvalidPoint);
        }
        for p in &original.points {
            validate(Point {
                x: p.x + offset.x,
                y: p.y + offset.y,
            })?;
        }
        for (point, source) in self.points.iter_mut().zip(&original.points) {
            *point = Point {
                x: source.x + offset.x,
                y: source.y + offset.y,
            };
        }
        Ok(())
    }
}
