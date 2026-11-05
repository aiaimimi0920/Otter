//! 整图翻转候选：UI 线程提供文字度量，后台翻转栅格并重算模糊，统一历史原子交付。
use super::*;
use crate::{
    erase_layers::Layers,
    raster_flip::{self, Axis},
};

#[derive(Debug)]
pub enum FlipError {
    Edit(EditError),
    Pixels(raster_flip::Error),
    Blur(crate::blur::BlurError),
}
pub struct FlipPlan {
    source: Layers,
    axis: Axis,
    annotation_size: (u32, u32),
    raster: RasterRefreshPlan,
}
impl FlipPlan {
    pub fn compute(self, cancelled: impl Fn() -> bool) -> Result<RasterRefreshPlan, FlipError> {
        // 主干底图翻转全源，标注层则重绘到当前可见框；两层全部成功才交付。
        let base = raster_flip::image(&self.source.base, self.source.size, self.axis, &cancelled)
            .map_err(FlipError::Pixels)?;
        let annotations = self
            .source
            .annotations
            .as_ref()
            .map(|pixels| {
                raster_flip::resized_image(
                    pixels,
                    self.source.annotation_dimensions(),
                    self.annotation_size,
                    self.axis,
                    &cancelled,
                )
            })
            .transpose()
            .map_err(FlipError::Pixels)?;
        let candidate = Layers {
            annotation_size: (annotations.is_some() && self.annotation_size != self.source.size)
                .then_some(self.annotation_size),
            size: self.source.size,
            base,
            annotations,
        };
        self.raster
            .with_candidate(candidate)
            .and_then(|plan| plan.compute(cancelled))
            .map_err(FlipError::Blur)
    }
}
impl Editor {
    pub fn prepare_flip(
        &self,
        axis: Axis,
        mut measure: impl FnMut(
            &crate::text_annotation::TextAnnotation,
        ) -> Result<TextFlipMetrics, EditError>,
    ) -> Result<Option<FlipPlan>, FlipError> {
        if self.draft.is_some() {
            return Ok(None);
        }
        let source = self
            .raster
            .as_ref()
            .ok_or(FlipError::Edit(EditError::InvalidPoint))?;
        let extra = self
            .elements
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>();
        if self.retained_points + extra > MAX_TOTAL_POINTS {
            return Err(FlipError::Edit(EditError::BudgetExceeded));
        }
        // 底图保持主干的“完整源图翻转后裁剪”；标注则围绕裁剪可见框镜像。
        // 原生几何保留源坐标，因此用 source.rect 而不是窗口位置或局部零原点。
        let frame = self
            .crop
            .map(|view| view.source.rect)
            .unwrap_or(crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: source.size.0 as f64,
                height: source.size.1 as f64,
            });
        let mut replacements = Vec::with_capacity(self.elements.len());
        for stroke in &self.elements {
            let metrics = if stroke.tool == Tool::Text {
                Some(
                    measure(
                        stroke
                            .text
                            .as_ref()
                            .ok_or(FlipError::Edit(EditError::InvalidPoint))?,
                    )
                    .map_err(FlipError::Edit)?,
                )
            } else {
                None
            };
            replacements.push(
                stroke
                    .flipped_with_text_metrics(frame, axis, metrics)
                    .map_err(FlipError::Edit)?,
            );
        }
        let raster =
            RasterRefreshPlan::for_transform(self, replacements).map_err(FlipError::Edit)?;
        Ok(Some(FlipPlan {
            source: source.clone(),
            axis,
            annotation_size: (
                frame.width.round().max(1.0) as u32,
                frame.height.round().max(1.0) as u32,
            ),
            raster,
        }))
    }
}

#[cfg(test)]
#[path = "editor_flip_crop_tests.rs"]
mod crop_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::with_layers(Layers {
            annotation_size: None,
            size: (32, 16),
            base: (0..512)
                .flat_map(|i| [(i % 32 * 4) as u8, 20, 40, 255])
                .collect::<Vec<_>>()
                .into(),
            annotations: Some([10, 20, 30, 64].repeat(512).into()),
        })
        .unwrap();
        editor.begin(Tool::Line, Point { x: 4.0, y: 5.0 }).unwrap();
        editor.extend(Point { x: 10.0, y: 8.0 }).unwrap();
        editor.commit();
        let blur = crate::blur_image::BlurImage::with_strength(32, 16, vec![128; 2048].into(), 2.4)
            .unwrap();
        editor.begin_blur(Point { x: 12.0, y: 8.0 }, blur).unwrap();
        editor.commit();
        editor
    }
    #[test]
    fn flip_commits_raster_geometry_and_blur_in_one_reversible_history() {
        let mut editor = document();
        let source = editor.layers().unwrap().clone();
        let before = editor.elements().to_vec();
        let allocations: Vec<_> = editor
            .elements()
            .iter()
            .map(|stroke| stroke.points.as_ptr())
            .collect();
        let revision = editor.revision();
        let plan = editor
            .prepare_flip(Axis::Horizontal, |_| unreachable!())
            .unwrap()
            .unwrap();
        let plan = plan.compute(|| false).unwrap();
        assert_eq!(editor.revision(), revision);
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.elements()[0].points[0], Point { x: 28.0, y: 5.0 });
        let expected = raster_flip::layers(&source, Axis::Horizontal, || false).unwrap();
        assert_eq!(&editor.layers().unwrap().base[..], &expected.base[..]);
        let composite = crate::raster_source::compose(&expected, || false).unwrap();
        let blur = crate::blur::region_precise(
            &composite,
            expected.size,
            crate::blur::Region {
                x: 0,
                y: 0,
                width: 32,
                height: 16,
            },
            2.4,
            || false,
        )
        .unwrap();
        assert_eq!(
            &editor.elements()[1].blur.as_ref().unwrap().pixels()[..],
            &blur[..]
        );
        for _ in 0..3 {
            assert!(editor.undo());
            assert!(editor.layers().unwrap().same_storage(&source));
            assert_eq!(
                editor
                    .elements()
                    .iter()
                    .map(|s| s.points.as_ptr())
                    .collect::<Vec<_>>(),
                allocations
            );
            assert_eq!(editor.elements()[0].points, before[0].points);
            assert!(editor.redo());
        }
    }
    #[test]
    fn text_measurement_failure_and_point_budget_leave_history_untouched() {
        let mut editor = document();
        for text in ["One", "Two"] {
            editor
                .insert_text(
                    Point { x: 1.0, y: 1.0 },
                    Point { x: 12.0, y: 10.0 },
                    crate::text_annotation::TextAnnotation::new(text, "Segoe UI", 10.0, false)
                        .unwrap(),
                )
                .unwrap();
        }
        let revision = editor.revision();
        let mut calls = 0;
        let result = editor.prepare_flip(Axis::Horizontal, |_| {
            calls += 1;
            if calls == 2 {
                Err(EditError::InvalidPoint)
            } else {
                Ok(TextFlipMetrics {
                    advance_width: 4.0,
                    left_inset: 4.0,
                    baseline_from_top: 8.0,
                })
            }
        });
        assert!(matches!(
            result,
            Err(FlipError::Edit(EditError::InvalidPoint))
        ));
        assert_eq!(calls, 2);
        assert_eq!(editor.revision(), revision);
        let retained = editor.retained_points;
        editor.retained_points = MAX_TOTAL_POINTS;
        assert!(matches!(
            editor.prepare_flip(Axis::Vertical, |_| unreachable!()),
            Err(FlipError::Edit(EditError::BudgetExceeded))
        ));
        editor.retained_points = retained;
        assert!(editor.undo());
        assert_eq!(editor.elements().len(), 3);
        assert!(editor.redo());
    }

    #[test]
    fn cancelled_stale_and_cropped_requests_never_partially_commit() {
        let mut editor = document();
        let plan = editor
            .prepare_flip(Axis::Vertical, |_| unreachable!())
            .unwrap()
            .unwrap();
        assert!(matches!(
            plan.compute(|| true),
            Err(FlipError::Pixels(raster_flip::Error::Cancelled))
        ));
        let source = editor.layers().unwrap().clone();
        let plan = editor
            .prepare_flip(Axis::Vertical, |_| unreachable!())
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        editor.undo();
        assert!(!editor.commit_raster_refresh(plan).unwrap());
        assert!(editor.layers().unwrap().same_storage(&source));
        assert!(editor.redo());
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 2.0,
                y: 2.0,
                width: 10.0,
                height: 10.0,
            })
            .unwrap();
        assert!(editor
            .prepare_flip(Axis::Horizontal, |_| unreachable!())
            .unwrap()
            .is_some());
    }
}
