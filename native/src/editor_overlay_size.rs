//! 将覆盖层映射到新生成图片的坐标；只生成快照，不写原文档历史或复制底图。
use super::*;

impl Editor {
    pub fn export_overlay_for_size(
        &self,
        size: (u32, u32),
        mut measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<Self, EditError> {
        if crate::viewport::checked_image_bytes(size.0, size.1).is_none() {
            return Err(EditError::InvalidPoint);
        }
        let mut snapshot = self.export_overlay_snapshot();
        let Some(source) = self.layers() else {
            return Ok(snapshot);
        };
        if source.size == size {
            return Ok(snapshot);
        }
        // 新生成图像使用新的像素坐标；不得保留旧恢复文档的完整逻辑框尺寸。
        snapshot.logical_size = None;
        let factors = Point {
            x: size.0 as f32 / source.size.0 as f32,
            y: size.1 as f32 / source.size.1 as f32,
        };
        let factor = (factors.x + factors.y) / 2.0;
        for (original, changed) in self.elements.iter().zip(&mut snapshot.elements) {
            if matches!(original.tool, Tool::Blur | Tool::Mosaic | Tool::Eraser) {
                return Err(EditError::InvalidPoint);
            }
            let measured = if original.tool == Tool::Text {
                let text = original
                    .text
                    .as_ref()
                    .and_then(|text| text.with_font_size(text.font_size() * factor))
                    .ok_or(EditError::BudgetExceeded)?;
                let measured = measure(&text)?;
                if !measured.x.is_finite()
                    || !measured.y.is_finite()
                    || measured.x <= 0.0
                    || measured.y <= 0.0
                {
                    return Err(EditError::InvalidPoint);
                }
                Some(measured)
            } else {
                None
            };
            changed.update_resize_preview(original, factors, measured, Some(Point::default()))?;
        }
        if let Some(crop) = &mut snapshot.crop {
            let scale = |r: crate::crop_geometry::Rect| crate::crop_geometry::Rect {
                x: r.x * factors.x as f64,
                y: r.y * factors.y as f64,
                width: r.width * factors.x as f64,
                height: r.height * factors.y as f64,
            };
            crop.frame = scale(crop.frame);
            crop.source.rect = scale(crop.source.rect);
            crop.source.source = crate::crop_geometry::Size {
                width: size.0 as f64,
                height: size.1 as f64,
            };
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
#[path = "editor_overlay_size_tests.rs"]
mod tests;
