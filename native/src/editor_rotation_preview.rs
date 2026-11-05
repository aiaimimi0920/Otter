//! 连续旋转复用预览点缓冲；始终相对原始几何计算，先完整校验再写入。
use super::*;

impl Stroke {
    pub(crate) fn update_rotation_preview(
        &mut self,
        original: &Self,
        pivot: Point,
        degrees: f32,
    ) -> Result<(), EditError> {
        validate(pivot)?;
        if !degrees.is_finite()
            || self.tool != original.tool
            || self.points.len() != original.points.len()
        {
            return Err(EditError::InvalidPoint);
        }
        let degrees = degrees.rem_euclid(360.0);
        let center = crate::rotation::center(&original.points);
        let effects = matches!(original.tool, Tool::Blur | Tool::Mosaic);
        let next = crate::rotation::rotate(center, pivot, degrees);
        let offset = Point {
            x: next.x - center.x,
            y: next.y - center.y,
        };
        let transformed = |point: Point| {
            if effects {
                crate::rotation::rotate(
                    crate::rotation::rotate(point, center, original.rotation),
                    pivot,
                    degrees,
                )
            } else {
                Point {
                    x: point.x + offset.x,
                    y: point.y + offset.y,
                }
            }
        };
        // 指针越界不能留下前半条笔迹已更新的预览。
        for &point in &original.points {
            validate(transformed(point))?;
        }
        for (point, source) in self.points.iter_mut().zip(&original.points) {
            *point = transformed(*source);
        }
        self.rotation = if effects {
            0.0
        } else {
            (original.rotation + degrees).rem_euclid(360.0)
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_preview_reuses_buffer_and_matches_commit_without_accumulation() {
        for tool in [Tool::Rectangle, Tool::Mosaic, Tool::Line] {
            let mut editor = Editor::default();
            editor.begin(tool, Point { x: 10.0, y: 10.0 }).unwrap();
            editor.extend(Point { x: 30.0, y: 20.0 }).unwrap();
            assert!(editor.commit());
            let original = editor.elements[0].clone();
            let mut preview = original.clone();
            let pointer = preview.points.as_ptr();
            let pivot = Point { x: 0.0, y: 0.0 };
            for angle in [15.0, 30.0, -45.0, 90.0] {
                preview
                    .update_rotation_preview(&original, pivot, angle)
                    .unwrap();
                assert_eq!(preview.points.as_ptr(), pointer);
            }
            assert!(editor.rotate_many_around(&[0], pivot, 90.0).unwrap());
            assert_eq!(preview.points, editor.elements[0].points);
            assert_eq!(preview.rotation, editor.elements[0].rotation);
            assert!(editor.undo());
            assert_eq!(editor.elements[0].points, original.points);
        }
    }
    #[test]
    fn failed_preview_preserves_previous_geometry_and_angle() {
        let mut editor = Editor::default();
        editor
            .begin(Tool::Mosaic, Point { x: 10.0, y: 10.0 })
            .unwrap();
        editor.extend(Point { x: 30.0, y: 20.0 }).unwrap();
        assert!(editor.commit());
        let original = editor.elements[0].clone();
        let mut preview = original.clone();
        preview
            .update_rotation_preview(&original, Point::default(), 30.0)
            .unwrap();
        let previous = preview.points.clone();
        for (pivot, angle) in [
            (Point::default(), f32::NAN),
            (
                Point {
                    x: -1_000_000.0,
                    y: 0.0,
                },
                180.0,
            ),
        ] {
            assert!(preview
                .update_rotation_preview(&original, pivot, angle)
                .is_err());
            assert_eq!(preview.points, previous);
            assert_eq!(preview.rotation, 0.0);
        }
    }
}
