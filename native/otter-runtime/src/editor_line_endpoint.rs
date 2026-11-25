//! 直线/箭头端点与折线顶点编辑；旋转中心变化不带动其他点。
use super::*;

fn valid(stroke: &Stroke, handle: usize) -> bool {
    handle < stroke.points.len()
        && match stroke.tool {
            Tool::Line | Tool::Arrow => stroke.points.len() == 2,
            Tool::Polyline => (2..=MAX_POINTS_PER_STROKE).contains(&stroke.points.len()),
            _ => false,
        }
}

impl Stroke {
    pub fn preview_line_endpoint(
        &self,
        handle: usize,
        point: Point,
    ) -> Result<Option<Self>, EditError> {
        validate(point)?;
        if !valid(self, handle) {
            return Ok(None);
        }
        let mut changed = self.clone();
        changed.points[handle] = point;
        crate::rotation::preserve_center(&self.points, &mut changed.points, self.rotation);
        for &point in &changed.points {
            validate(point)?;
        }
        Ok(Some(changed))
    }
}

impl Editor {
    pub fn move_line_endpoint(
        &mut self,
        index: usize,
        handle: usize,
        point: Point,
    ) -> Result<bool, EditError> {
        validate(point)?;
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(source) = self.elements.get(index) else {
            return Ok(false);
        };
        if !valid(source, handle) || source.points[handle] == point {
            return Ok(false);
        }
        let count = source.points.len();
        if self.retained_points + count > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let changed = source.preview_line_endpoint(handle, point)?.unwrap();
        let old = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += count;
        self.retained_points -= self.history.record(Entry::replaced(index, old));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_or_over_budget_endpoint_edit_preserves_document() {
        let mut editor = Editor::default();
        editor.begin(Tool::Line, Point::default()).unwrap();
        editor.extend(Point { x: 10.0, y: 10.0 }).unwrap();
        editor.commit();
        let before = editor.elements()[0].points.clone();
        let revision = editor.revision();
        assert!(!editor.move_line_endpoint(0, 2, Point::default()).unwrap());
        assert_eq!(
            editor.move_line_endpoint(
                0,
                0,
                Point {
                    x: f32::NAN,
                    y: 0.0
                }
            ),
            Err(EditError::InvalidPoint)
        );
        editor.retained_points = MAX_TOTAL_POINTS - 1;
        assert_eq!(
            editor.move_line_endpoint(0, 1, Point { x: 20.0, y: 20.0 }),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(editor.elements()[0].points, before);
        assert_eq!(editor.revision(), revision);
    }
}
