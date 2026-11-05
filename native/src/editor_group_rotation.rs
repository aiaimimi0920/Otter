//! 组旋转事务；调用方提供视觉组中心，效果路径旋转几何而不旋转源图采样空间。
use super::*;

impl Editor {
    pub fn rotate_many_around(
        &mut self,
        indices: &[usize],
        pivot: Point,
        degrees: f32,
    ) -> Result<bool, EditError> {
        self.rotate_many_mode(indices, pivot, degrees, false)
    }

    pub fn rotate_many_mode(
        &mut self,
        indices: &[usize],
        pivot: Point,
        degrees: f32,
        own: bool,
    ) -> Result<bool, EditError> {
        validate(pivot)?;
        if !degrees.is_finite()
            || indices.iter().any(|index| *index >= self.elements.len())
            || !indices.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EditError::InvalidPoint);
        }
        let degrees = degrees.rem_euclid(360.0);
        if indices.is_empty() || degrees == 0.0 || self.draft.is_some() {
            return Ok(false);
        }
        let extra: usize = indices
            .iter()
            .map(|index| self.elements[*index].points.len())
            .sum();
        if self.retained_points + extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let mut replacements = Vec::with_capacity(indices.len());
        for &index in indices {
            let original = &self.elements[index];
            let pivot = if own {
                crate::group_bounds::bounds(original)
                    .ok_or(EditError::InvalidPoint)?
                    .center()
            } else {
                pivot
            };
            let mut changed = original.clone();
            changed.update_rotation_preview(original, pivot, degrees)?;
            replacements.push((index, changed));
        }
        // 候选全部通过才交换；失败时原文档、版本、历史及重做分支均不变。
        for (index, changed) in &mut replacements {
            std::mem::swap(changed, &mut self.elements[*index]);
        }
        self.retained_points += extra;
        self.retained_points -= self.history.record(Entry::replaced_many(replacements));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::default();
        for (tool, x) in [(Tool::Rectangle, 10.0), (Tool::Mosaic, 30.0)] {
            editor.begin(tool, Point { x, y: 10.0 }).unwrap();
            editor
                .extend(Point {
                    x: x + 4.0,
                    y: 14.0,
                })
                .unwrap();
            assert!(editor.commit());
        }
        editor
    }
    fn near(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
        assert!(
            (actual.y - expected.y).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
    }
    #[test]
    fn shape_and_effect_rotate_about_shared_pivot_and_undo_exactly() {
        let mut editor = document();
        let pointers: Vec<_> = editor.elements.iter().map(|s| s.points.as_ptr()).collect();
        let pivot = Point { x: 22.0, y: 12.0 };
        let revision = editor.revision();
        assert!(editor.rotate_many_around(&[0, 1], pivot, 90.0).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.elements[0].rotation, 90.0);
        assert_eq!(editor.elements[1].rotation, 0.0);
        near(
            crate::rotation::center(&editor.elements[0].points),
            Point { x: 22.0, y: 2.0 },
        );
        near(editor.elements[1].points[0], Point { x: 24.0, y: 20.0 });
        let transformed = editor.elements[1].points.clone();
        for _ in 0..3 {
            assert!(editor.undo());
            assert_eq!(
                editor
                    .elements
                    .iter()
                    .map(|s| s.points.as_ptr())
                    .collect::<Vec<_>>(),
                pointers
            );
            assert!(editor.redo());
            assert_eq!(editor.elements[1].points, transformed);
        }
    }
    #[test]
    fn failures_and_noops_preserve_revision_and_redo() {
        let mut editor = document();
        editor
            .translate_many(&[0, 1], Point { x: 1.0, y: 0.0 })
            .unwrap();
        assert!(editor.undo());
        let revision = editor.revision();
        assert!(editor
            .rotate_many_around(
                &[0, 1],
                Point {
                    x: -1_000_000.0,
                    y: 0.0
                },
                180.0
            )
            .is_err());
        for indices in [&[0, 0][..], &[1, 0], &[0, 2]] {
            assert!(editor
                .rotate_many_around(indices, Point::default(), 90.0)
                .is_err());
        }
        assert!(editor
            .rotate_many_around(&[0, 1], Point::default(), f32::NAN)
            .is_err());
        assert!(!editor
            .rotate_many_around(&[0, 1], Point::default(), 360.0)
            .unwrap());
        assert_eq!(editor.revision(), revision);
        assert_eq!(editor.elements[0].points[0], Point { x: 10.0, y: 10.0 });
        assert!(editor.redo());
        editor.retained_points = MAX_TOTAL_POINTS;
        assert!(matches!(
            editor.rotate_many_around(&[0, 1], Point::default(), 90.0),
            Err(EditError::BudgetExceeded)
        ));
    }
    #[test]
    fn blur_rotation_shares_pixels_and_folds_old_angle_into_mask() {
        let mut editor = Editor::default();
        let pixels: crate::pixels::Pixels = vec![255; 16 * 16 * 4].into();
        let blur = crate::blur_image::BlurImage::new(16, 16, pixels.clone(), 8).unwrap();
        editor.begin_blur(Point { x: 2.0, y: 4.0 }, blur).unwrap();
        editor.extend(Point { x: 6.0, y: 4.0 }).unwrap();
        assert!(editor.commit());
        assert!(editor.rotate(0, 90.0).unwrap());
        let pivot = Point { x: 4.0, y: 4.0 };
        assert!(editor.rotate_many_around(&[0], pivot, 90.0).unwrap());
        near(editor.elements[0].points[0], Point { x: 6.0, y: 4.0 });
        assert_eq!(editor.elements[0].rotation, 0.0);
        assert!(editor.elements[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&pixels));
        assert!(editor.undo());
        assert_eq!(editor.elements[0].rotation, 90.0);
        assert_eq!(editor.elements[0].points[0], Point { x: 2.0, y: 4.0 });
        assert!(editor.elements[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&pixels));
    }
    #[test]
    fn own_center_rotation_keeps_each_visual_center_and_undoes_once() {
        let mut editor = document();
        let centers: Vec<_> = editor
            .elements
            .iter()
            .map(|s| crate::group_bounds::bounds(s).unwrap().center())
            .collect();
        let revision = editor.revision();
        assert!(editor
            .rotate_many_mode(&[0, 1], Point::default(), 90.0, true)
            .unwrap());
        assert_eq!(editor.revision(), revision + 1);
        for (stroke, center) in editor.elements.iter().zip(centers) {
            near(
                crate::group_bounds::bounds(stroke).unwrap().center(),
                center,
            );
        }
        assert!(editor.undo());
        assert_eq!(editor.elements[0].rotation, 0.0);
        assert_eq!(editor.elements[1].points[0], Point { x: 30.0, y: 10.0 });
    }
}
