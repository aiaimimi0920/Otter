//! 组缩放先完成文字排版及全部候选，随后一次提交；路径按图像轴变换，形状保留局部角度。
use super::*;

/// 未提交候选独占几何缓冲、共享图像；后台计算可补充效果，不能直接改动文档。
pub struct ResizePlan {
    scale: f32,
    identity: std::sync::Arc<()>,
    revision: u64,
    source: Option<crate::erase_layers::Layers>,
    annotation_frame: Option<crate::crop_geometry::Rect>,
    base_projection: Option<crate::image_projection::Projection>,
    replacements: Vec<(usize, Stroke)>,
    extra: usize,
}

impl ResizePlan {
    pub fn base_projection(&self) -> Option<crate::image_projection::Projection> {
        self.base_projection
    }
    pub fn annotation_frame(&self) -> Option<crate::crop_geometry::Rect> {
        self.annotation_frame
    }
    pub fn source_layers(&self) -> Option<&crate::erase_layers::Layers> {
        self.source.as_ref()
    }
    pub fn blur_targets(&self) -> Result<Vec<(usize, f32)>, EditError> {
        self.replacements
            .iter()
            .filter(|(_, stroke)| stroke.tool == Tool::Blur)
            .map(|(index, stroke)| {
                let strength = stroke
                    .blur
                    .as_ref()
                    .ok_or(EditError::InvalidPoint)?
                    .strength()
                    * self.scale;
                if !strength.is_finite() || !(1.0..=64.0).contains(&strength) {
                    return Err(EditError::BudgetExceeded);
                }
                Ok((*index, strength))
            })
            .collect()
    }
    pub fn strokes(&self) -> impl Iterator<Item = (usize, &Stroke)> {
        self.replacements
            .iter()
            .map(|(index, stroke)| (*index, stroke))
    }
    pub fn replace_blur(
        &mut self,
        index: usize,
        image: crate::blur_image::BlurImage,
    ) -> Result<(), EditError> {
        let (_, stroke) = self
            .replacements
            .iter_mut()
            .find(|(target, _)| *target == index)
            .ok_or(EditError::InvalidPoint)?;
        let original = stroke.blur.as_ref().ok_or(EditError::InvalidPoint)?;
        if stroke.tool != Tool::Blur
            || image.width() != original.width()
            || image.height() != original.height()
        {
            return Err(EditError::InvalidPoint);
        }
        stroke.blur = Some(image);
        Ok(())
    }
}

impl Editor {
    pub fn resize_many_around(
        &mut self,
        indices: &[usize],
        pivot: Point,
        factor: f32,
        measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<bool, EditError> {
        self.resize_many_axes_around(
            indices,
            pivot,
            Point {
                x: factor,
                y: factor,
            },
            measure,
        )
    }

    pub fn resize_many_axes_around(
        &mut self,
        indices: &[usize],
        pivot: Point,
        factors: Point,
        measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<bool, EditError> {
        let Some(plan) = self.prepare_resize_many(indices, pivot, factors, measure)? else {
            return Ok(false);
        };
        self.commit_resize(plan)
    }

    pub fn prepare_resize_many(
        &self,
        indices: &[usize],
        pivot: Point,
        factors: Point,
        measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<Option<ResizePlan>, EditError> {
        self.prepare_resize_many_mode(indices, pivot, factors, false, measure)
    }

    pub fn prepare_resize_many_mode(
        &self,
        indices: &[usize],
        pivot: Point,
        factors: Point,
        own: bool,
        mut measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<Option<ResizePlan>, EditError> {
        validate(pivot)?;
        if !factors.x.is_finite()
            || !factors.y.is_finite()
            || factors.x <= 0.0
            || factors.y <= 0.0
            || indices.iter().any(|index| *index >= self.elements.len())
            || !indices.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EditError::InvalidPoint);
        }
        if indices.is_empty() || factors == (Point { x: 1.0, y: 1.0 }) || self.draft.is_some() {
            return Ok(None);
        }
        let extra: usize = indices
            .iter()
            .map(|index| self.elements[*index].points.len())
            .sum();
        if self.retained_points + extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let mut replacements = Vec::with_capacity(indices.len());
        let factor = (factors.x + factors.y) / 2.0;
        for &index in indices {
            let original = &self.elements[index];
            let pivot = if own {
                crate::group_bounds::bounds(original)
                    .ok_or(EditError::InvalidPoint)?
                    .center()
            } else {
                pivot
            };
            let measured = if original.tool == Tool::Text {
                let text = original
                    .text
                    .as_ref()
                    .and_then(|text| text.with_font_size(text.font_size() * factor))
                    .ok_or(EditError::BudgetExceeded)?;
                let size = measure(&text)?;
                if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
                    return Err(EditError::InvalidPoint);
                }
                Some(size)
            } else {
                None
            };
            let mut changed = original.clone();
            changed.update_resize_preview(original, factors, measured, Some(pivot))?;
            replacements.push((index, changed));
        }
        Ok(Some(ResizePlan {
            scale: factor,
            identity: self.identity.clone(),
            revision: self.revision,
            source: self.raster.clone(),
            annotation_frame: self.annotation_frame(),
            base_projection: self.raster_base_projection(),
            replacements,
            extra,
        }))
    }

    pub fn commit_resize(&mut self, mut plan: ResizePlan) -> Result<bool, EditError> {
        let same_source = match (&self.raster, &plan.source) {
            (Some(current), Some(source)) => current.same_storage(source),
            (None, None) => true,
            _ => false,
        };
        if !std::sync::Arc::ptr_eq(&self.identity, &plan.identity)
            || self.revision != plan.revision
            || self.annotation_frame() != plan.annotation_frame
            || self.draft.is_some()
            || !same_source
        {
            return Ok(false);
        }
        if self.retained_points + plan.extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        for (index, changed) in &mut plan.replacements {
            std::mem::swap(changed, &mut self.elements[*index]);
        }
        self.retained_points += plan.extra;
        self.retained_points -= self.history.record(Entry::replaced_many(plan.replacements));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::default();
        for x in [10.0, 30.0] {
            editor.begin(Tool::Rectangle, Point { x, y: 10.0 }).unwrap();
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
    #[test]
    fn group_scale_moves_centers_scales_style_and_restores_allocations() {
        let mut editor = document();
        let pointers: Vec<_> = editor.elements.iter().map(|s| s.points.as_ptr()).collect();
        let width = editor.elements[0].width;
        let revision = editor.revision();
        assert!(editor
            .resize_many_around(&[0, 1], Point { x: 22.0, y: 12.0 }, 2.0, |_| panic!(
                "no text"
            ))
            .unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.elements[0].points[0], Point { x: -2.0, y: 8.0 });
        assert_eq!(editor.elements[1].points[1], Point { x: 46.0, y: 16.0 });
        assert_eq!(editor.elements[0].width, width * 2.0);
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
        assert_eq!(editor.elements[0].points[0], Point { x: -2.0, y: 8.0 });
    }
    #[test]
    fn anisotropic_paths_scale_in_image_axes_while_shapes_keep_local_rotation() {
        let mut editor = document();
        editor
            .begin(Tool::Line, Point { x: 10.0, y: 10.0 })
            .unwrap();
        editor.extend(Point { x: 14.0, y: 10.0 }).unwrap();
        assert!(editor.commit());
        editor.rotate(0, 90.0).unwrap();
        editor.rotate(2, 90.0).unwrap();
        let revision = editor.revision();
        assert!(editor
            .resize_many_axes_around(
                &[0, 2],
                Point::default(),
                Point { x: 2.0, y: 3.0 },
                |_| panic!("no text")
            )
            .unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.elements[0].rotation, 90.0);
        assert_eq!(editor.elements[2].rotation, 0.0);
        let points = &editor.elements[2].points;
        assert!((points[0].x - 24.0).abs() < 0.001);
        assert!((points[0].y - 24.0).abs() < 0.001);
        assert!((points[1].x - 24.0).abs() < 0.001);
        assert!((points[1].y - 36.0).abs() < 0.001);
        assert!(editor.undo());
        assert_eq!(editor.elements[2].rotation, 90.0);
        assert_eq!(editor.elements[2].points[0], Point { x: 10.0, y: 10.0 });
    }
    #[test]
    fn failed_candidate_preserves_whole_document_and_redo() {
        let mut editor = document();
        editor
            .translate_many(&[0, 1], Point { x: 1.0, y: 0.0 })
            .unwrap();
        assert!(editor.undo());
        let revision = editor.revision();
        for factor in [0.0, -1.0, f32::NAN, f32::INFINITY, 100_000.0] {
            assert!(editor
                .resize_many_around(&[0, 1], Point::default(), factor, |_| panic!("no text"))
                .is_err());
        }
        assert_eq!(editor.revision(), revision);
        assert_eq!(editor.elements[0].points[0], Point { x: 10.0, y: 10.0 });
        assert!(editor.redo());
    }
    #[test]
    fn text_measurement_failure_is_atomic_and_success_uses_measured_not_stretched_size() {
        let mut editor = document();
        let text =
            crate::text_annotation::TextAnnotation::new("测试", "微软雅黑", 16.0, false).unwrap();
        editor
            .insert_text(Point { x: 40.0, y: 10.0 }, Point { x: 20.0, y: 20.0 }, text)
            .unwrap();
        let revision = editor.revision();
        let before = editor.elements[0].points.clone();
        assert!(editor
            .resize_many_around(&[0, 2], Point::default(), 2.0, |_| Err(
                EditError::InvalidPoint
            ))
            .is_err());
        assert_eq!(editor.revision(), revision);
        assert_eq!(editor.elements[0].points, before);
        assert!(editor
            .resize_many_around(&[0, 2], Point::default(), 2.0, |text| {
                assert_eq!(text.font_size(), 32.0);
                Ok(Point { x: 43.0, y: 38.0 })
            })
            .unwrap());
        let text = &editor.elements[2];
        assert_eq!(text.points[1].x - text.points[0].x, 43.0);
        assert_eq!(text.points[1].y - text.points[0].y, 38.0);
        assert_eq!(
            crate::rotation::center(&text.points),
            Point { x: 100.0, y: 40.0 }
        );
        assert!(editor.undo());
        assert_eq!(editor.elements[2].text.as_ref().unwrap().font_size(), 16.0);
    }
    #[test]
    fn prepared_resize_never_mutates_document_and_rejects_stale_or_other_documents() {
        let mut editor = document();
        let revision = editor.revision();
        let prepare = |editor: &Editor| {
            editor
                .prepare_resize_many(&[0, 1], Point::default(), Point { x: 2.0, y: 2.0 }, |_| {
                    panic!("no text")
                })
                .unwrap()
                .unwrap()
        };
        let plan = prepare(&editor);
        assert_eq!(editor.revision(), revision);
        assert_eq!(editor.elements[0].points[0], Point { x: 10.0, y: 10.0 });
        let mut other = document();
        assert_eq!(other.revision(), revision);
        assert!(!other.commit_resize(plan).unwrap());
        let plan = prepare(&editor);
        assert!(editor.undo());
        assert!(!editor.commit_resize(plan).unwrap());
        assert!(editor.redo());
        let plan = prepare(&editor);
        editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(!editor.commit_resize(plan).unwrap());
        assert!(editor.cancel());
        assert!(editor.commit_resize(prepare(&editor)).unwrap());
    }
    #[test]
    fn prepared_blur_replacement_and_geometry_commit_together_and_undo_together() {
        let mut editor = document();
        let old_pixels: crate::pixels::Pixels = vec![128; 16 * 16 * 4].into();
        let old = crate::blur_image::BlurImage::new(16, 16, old_pixels.clone(), 8).unwrap();
        editor.begin_blur(Point { x: 2.0, y: 2.0 }, old).unwrap();
        editor.extend(Point { x: 6.0, y: 2.0 }).unwrap();
        assert!(editor.commit());
        let revision = editor.revision();
        let mut plan = editor
            .prepare_resize_many(&[0, 2], Point::default(), Point { x: 2.0, y: 2.0 }, |_| {
                panic!("no text")
            })
            .unwrap()
            .unwrap();
        let new_pixels: crate::pixels::Pixels = vec![64; 16 * 16 * 4].into();
        let new = crate::blur_image::BlurImage::new(16, 16, new_pixels.clone(), 16).unwrap();
        assert!(plan.replace_blur(0, new.clone()).is_err());
        assert!(plan.replace_blur(99, new.clone()).is_err());
        let wrong = crate::blur_image::BlurImage::new(8, 8, vec![0; 8 * 8 * 4].into(), 16).unwrap();
        assert!(plan.replace_blur(2, wrong).is_err());
        plan.replace_blur(2, new).unwrap();
        assert_eq!(editor.elements[2].blur.as_ref().unwrap().sigma(), 8);
        assert!(editor.commit_resize(plan).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.elements[2].points[0], Point { x: 4.0, y: 4.0 });
        assert!(editor.elements[2]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&new_pixels));
        assert!(editor.undo());
        assert_eq!(editor.elements[2].points[0], Point { x: 2.0, y: 2.0 });
        assert!(editor.elements[2]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&old_pixels));
        assert!(editor.redo());
        assert!(editor.elements[2]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&new_pixels));
    }
}
