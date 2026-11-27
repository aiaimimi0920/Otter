//! Horizontal handles reflow text without scaling its font. The opposing edge
//! remains in place even for a rotated label; measurement is supplied by the host.
use super::*;
impl Stroke {
    pub fn text_handles(&self) -> Option<[Point; 2]> {
        if self.tool != Tool::Text
            || self.text.as_ref()?.geometry().is_none()
            || self.points.len() != 2
        {
            return None;
        }
        let (a, b) = (self.points[0], self.points[1]);
        let y = (a.y + b.y) * 0.5;
        Some([Point { x: a.x, y }, Point { x: b.x, y }])
    }
    pub fn preview_text_handle(
        &self,
        side: usize,
        x: f32,
        measure: impl FnOnce(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<Option<Self>, EditError> {
        let Some(handles) = self.text_handles() else {
            return Ok(None);
        };
        if side >= 2 || !x.is_finite() {
            return Err(EditError::InvalidPoint);
        }
        let text = self.text.as_ref().unwrap();
        let w = if side == 0 {
            handles[1].x - x
        } else {
            x - handles[0].x
        };
        let b =
            crate::text_annotation::TextBox::new(w.max(2. * text.padding() + 1.), text.padding())
                .ok_or(EditError::InvalidPoint)?;
        let next = text.clone().with_geometry(Some(b));
        let size = measure(&next)?;
        if size.x != b.width() || !size.y.is_finite() || size.y <= 0. {
            return Err(EditError::InvalidPoint);
        }
        let mut out = self.clone();
        if side == 0 {
            out.points[0].x = out.points[1].x - size.x;
        } else {
            out.points[1].x = out.points[0].x + size.x;
        }
        out.points[1].y = out.points[0].y + size.y;
        crate::rotation::preserve_center(&self.points, &mut out.points, self.rotation);
        for &p in &out.points {
            validate(p)?;
        }
        out.text = Some(next);
        Ok(Some(out))
    }
}
impl Editor {
    pub fn move_text_handle(
        &mut self,
        index: usize,
        side: usize,
        x: f32,
        measure: impl FnOnce(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<bool, EditError> {
        let Some(source) = self.elements.get(index) else {
            return Ok(false);
        };
        let Some(next) = source.preview_text_handle(side, x, measure)? else {
            return Ok(false);
        };
        self.commit_label_preview(index, next)
    }
}
