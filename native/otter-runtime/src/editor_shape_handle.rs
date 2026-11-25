//! Four-corner box-annotation adjustment; fix opposite corners, match baseline minimum dimensions, and do not scale styles with handles.
use super::*;

impl Stroke {
    /// Order is top-left, top-right, bottom-left, bottom-right, independent of creation drag direction.
    pub fn shape_handles(&self) -> Option<[Point; 4]> {
        if !crate::shape_options::ShapeOptions::supports(self.tool) || self.points.len() != 2 {
            return None;
        }
        let a = self.points[0];
        let b = self.points[1];
        let (left, right, top, bottom) = (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
        Some([
            Point { x: left, y: top },
            Point { x: right, y: top },
            Point { x: left, y: bottom },
            Point {
                x: right,
                y: bottom,
            },
        ])
    }
    pub fn preview_shape_handle(
        &self,
        handle: usize,
        point: Point,
    ) -> Result<Option<Self>, EditError> {
        validate(point)?;
        let Some(handles) = self.shape_handles() else {
            return Ok(None);
        };
        if handle >= 4 {
            return Ok(None);
        }
        let opposite = handles[3 - handle];
        let moved = Point {
            x: if handle.is_multiple_of(2) {
                point.x.min(opposite.x - 16.0)
            } else {
                point.x.max(opposite.x + 16.0)
            },
            y: if handle < 2 {
                point.y.min(opposite.y - 16.0)
            } else {
                point.y.max(opposite.y + 16.0)
            },
        };
        let mut changed = self.clone();
        changed.points.copy_from_slice(&[
            Point {
                x: moved.x.min(opposite.x),
                y: moved.y.min(opposite.y),
            },
            Point {
                x: moved.x.max(opposite.x),
                y: moved.y.max(opposite.y),
            },
        ]);
        // Compensate rotation offsets caused by new centers so fixed opposite corners retain original world coordinates.
        crate::rotation::preserve_center(&self.points, &mut changed.points, self.rotation);
        for &point in &changed.points {
            validate(point)?;
        }
        Ok(Some(changed))
    }
}

impl Editor {
    pub fn move_shape_handle(
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
        let Some(handles) = source.shape_handles() else {
            return Ok(false);
        };
        if handle >= 4 || handles[handle] == point {
            return Ok(false);
        }
        if self.retained_points + source.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let changed = source.preview_shape_handle(handle, point)?.unwrap();
        if changed.points == source.points {
            return Ok(false);
        }
        let previous = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += previous.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, previous));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corners_obey_minimum_size_and_preserve_style_and_history() {
        for tool in [
            Tool::Rectangle,
            Tool::RoundedRectangle,
            Tool::Ellipse,
            Tool::Triangle,
            Tool::Polygon,
        ] {
            let mut editor = Editor::default();
            editor.begin(tool, Point { x: 80.0, y: 70.0 }).unwrap();
            editor.extend(Point { x: 20.0, y: 10.0 }).unwrap();
            editor.commit();
            let original = editor.elements()[0].clone();
            for handle in 0..4 {
                let opposite = original.shape_handles().unwrap()[3 - handle];
                assert!(editor.move_shape_handle(0, handle, opposite).unwrap());
                let changed = &editor.elements()[0];
                assert_eq!(changed.points[1].x - changed.points[0].x, 16.0);
                assert_eq!(changed.points[1].y - changed.points[0].y, 16.0);
                assert_eq!(changed.style(), original.style());
                assert!(editor.undo());
                assert_eq!(editor.elements()[0].points, original.points);
            }
            let revision = editor.revision();
            assert!(!editor.move_shape_handle(0, 4, Point::default()).unwrap());
            editor.retained_points = MAX_TOTAL_POINTS;
            assert_eq!(
                editor.move_shape_handle(0, 0, Point::default()),
                Err(EditError::BudgetExceeded)
            );
            assert_eq!(editor.revision(), revision);
            assert_eq!(editor.elements()[0].points, original.points);
        }
    }
}
