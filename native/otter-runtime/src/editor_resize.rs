//! 局部轴缩放只在提交时复制目标标注，原几何留在有界历史中精确恢复。
use super::*;

impl Editor {
    pub fn resize(&mut self, index: usize, factor: f32) -> Result<bool, EditError> {
        self.resize_axes(
            index,
            Point {
                x: factor,
                y: factor,
            },
        )
    }

    pub fn resize_axes(&mut self, index: usize, factors: Point) -> Result<bool, EditError> {
        self.resize_with_layout(index, factors, None)
    }

    /// 宿主按目标字号测量后提交；排版与几何必须在同一个历史事务中生效。
    pub fn resize_text_axes(
        &mut self,
        index: usize,
        factors: Point,
        measured_size: Point,
    ) -> Result<bool, EditError> {
        if !measured_size.x.is_finite()
            || !measured_size.y.is_finite()
            || measured_size.x <= 0.0
            || measured_size.y <= 0.0
        {
            return Err(EditError::InvalidPoint);
        }
        if !self
            .elements
            .get(index)
            .is_some_and(|s| s.tool == Tool::Text && s.points.len() == 2 && s.text.is_some())
        {
            return Ok(false);
        }
        self.resize_with_layout(index, factors, Some(measured_size))
    }

    fn resize_with_layout(
        &mut self,
        index: usize,
        factors: Point,
        measured_size: Option<Point>,
    ) -> Result<bool, EditError> {
        if !factors.x.is_finite() || !factors.y.is_finite() || factors.x <= 0.0 || factors.y <= 0.0
        {
            return Err(EditError::InvalidPoint);
        }
        if factors == (Point { x: 1.0, y: 1.0 }) || self.draft.is_some() {
            return Ok(false);
        }
        let Some(stroke) = self.elements.get(index) else {
            return Ok(false);
        };
        if self.retained_points + stroke.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let resized = resized_stroke(stroke, factors, measured_size)?;
        let original = std::mem::replace(&mut self.elements[index], resized);
        self.retained_points += original.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, original));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

pub(super) fn resized_stroke(
    stroke: &Stroke,
    factors: Point,
    measured_size: Option<Point>,
) -> Result<Stroke, EditError> {
    let mut resized = stroke.clone();
    resized.update_resize_preview(stroke, factors, measured_size, None)?;
    Ok(resized)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mosaic_resize_scales_fractional_cell_and_preserves_exact_history() {
        let mut editor = Editor::default();
        editor.set_style(
            editor
                .style()
                .with_width(20.0)
                .unwrap()
                .with_mosaic_cell(64)
                .unwrap(),
        );
        editor
            .begin(Tool::Mosaic, Point { x: 10.0, y: 20.0 })
            .unwrap();
        editor.extend(Point { x: 30.0, y: 40.0 }).unwrap();
        editor.commit();
        let before = editor.elements()[0].clone();
        editor.resize_axes(0, Point { x: 3.0, y: 1.0 }).unwrap();
        assert_eq!(editor.elements()[0].mosaic_cell, 128.0);
        assert_eq!(editor.elements()[0].width, 40.0);
        assert_eq!(editor.elements()[0].style().mosaic_cell(), 128.0);
        editor.undo();
        assert_eq!(editor.elements()[0].style(), before.style());
        assert_eq!(editor.elements()[0].points, before.points);
        editor.redo();
        assert_eq!(editor.elements()[0].mosaic_cell, 128.0);
        editor.resize(0, 0.1).unwrap();
        assert!((editor.elements()[0].mosaic_cell - 12.8).abs() < 0.0001);
        let revision = editor.revision();
        let style = editor.elements()[0]
            .style()
            .with_mosaic_cell_value(1_000_000.0)
            .unwrap();
        editor.restyle(0, style);
        let changed_revision = editor.revision();
        assert!(changed_revision > revision);
        assert_eq!(editor.resize(0, 2.0), Err(EditError::BudgetExceeded));
        assert_eq!(editor.revision(), changed_revision);
        assert_eq!(editor.elements()[0].mosaic_cell, 1_000_000.0);
    }

    #[test]
    fn measured_text_resize_rejects_invalid_layout_without_history() {
        let mut editor = Editor::default();
        editor
            .insert_text(
                Point { x: 20.0, y: 30.0 },
                Point { x: 60.0, y: 24.0 },
                crate::text_annotation::TextAnnotation::new("文字", "微软雅黑", 16.0, false)
                    .unwrap(),
            )
            .unwrap();
        let before = editor.elements()[0].clone();
        let revision = editor.revision();
        for size in [
            Point {
                x: f32::NAN,
                y: 20.0,
            },
            Point { x: 0.0, y: 20.0 },
            Point { x: 20.0, y: -1.0 },
            Point {
                x: 3_000_000.0,
                y: 20.0,
            },
        ] {
            assert!(editor
                .resize_text_axes(0, Point { x: 2.0, y: 0.5 }, size)
                .is_err());
            assert_eq!(editor.elements()[0].points, before.points);
            assert_eq!(editor.elements()[0].text, before.text);
            assert_eq!(editor.revision(), revision);
            assert_eq!(editor.retained_points, 2);
        }
        assert!(editor.undo());
        assert!(editor.elements().is_empty());
    }

    #[test]
    fn serial_axis_resize_keeps_circle_label_counter_and_exact_history() {
        let mut editor = Editor::default();
        editor
            .insert_serial(Point { x: 40.0, y: 60.0 }, 14.0, "微软雅黑")
            .unwrap();
        editor.rotate(0, 35.0).unwrap();
        let before = editor.elements()[0].clone();
        assert!(editor.resize_axes(0, Point { x: 2.0, y: 0.5 }).unwrap());
        let after = editor.elements()[0].clone();
        assert_eq!(
            after.points,
            vec![Point { x: 22.5, y: 42.5 }, Point { x: 57.5, y: 77.5 }]
        );
        assert_eq!(after.rotation, 35.0);
        assert_eq!(after.text.as_ref().unwrap().content(), "1");
        assert_eq!(after.text.as_ref().unwrap().font_size(), 20.0);
        assert_eq!(after.width, before.width * 1.25);
        assert_eq!(editor.serial_count, 1);
        assert!(editor.undo());
        assert_eq!(editor.elements()[0].points, before.points);
        assert_eq!(editor.elements()[0].text, before.text);
        assert_eq!(editor.elements()[0].style(), before.style());
        assert!(editor.redo());
        assert_eq!(editor.elements()[0].points, after.points);
        assert_eq!(editor.elements()[0].text, after.text);
        assert_eq!(editor.serial_count, 1);
    }

    #[test]
    fn serial_font_limit_rejects_resize_without_partial_mutation() {
        let mut editor = Editor::default();
        editor
            .insert_serial(Point { x: 0.0, y: 0.0 }, 999.0, "微软雅黑")
            .unwrap();
        let before = editor.elements()[0].clone();
        // 变换不受普通UI创建上限1149限制，但仍受恢复域1_000_000硬上限约束。
        assert!(editor.resize(0, 1.01).unwrap());
        assert!((editor.elements()[0].text.as_ref().unwrap().font_size() - 1160.49).abs() < 0.001);
        assert!(editor.undo());
        let revision = editor.revision();
        assert_eq!(editor.resize(0, 1000.0), Err(EditError::BudgetExceeded));
        assert_eq!(editor.elements()[0].points, before.points);
        assert_eq!(editor.elements()[0].text, before.text);
        assert_eq!(editor.elements()[0].style(), before.style());
        assert_eq!(editor.revision(), revision);
        assert_eq!(editor.serial_count, 1);
        // 当前标注和刚撤销的缩放redo各持2点；拒绝不能提前释放仍在历史中的owner。
        assert_eq!(editor.retained_points, 4);
        assert!(editor.undo());
        assert!(editor.elements().is_empty());
        assert_eq!(editor.serial_count, 0);
    }

    #[test]
    fn axis_resize_keeps_center_rotation_and_uses_average_style_factor() {
        let mut editor = document();
        editor.rotate(0, 45.0).unwrap();
        let old = editor.elements()[0].clone();
        assert!(editor.resize_axes(0, Point { x: 2.0, y: 0.5 }).unwrap());
        let resized = &editor.elements()[0];
        assert_eq!(
            resized.points,
            vec![Point { x: 0.0, y: 30.0 }, Point { x: 40.0, y: 50.0 }]
        );
        assert_eq!(resized.width, 5.0);
        assert_eq!(resized.corner_radius, 15.0);
        assert_eq!(resized.rotation, 45.0);
        editor.undo();
        assert_eq!(editor.elements()[0].points, old.points);
        assert_eq!(editor.elements()[0].style(), old.style());
        assert_eq!(editor.elements()[0].rotation, old.rotation);
    }

    #[test]
    fn invalid_axis_does_not_commit_the_other_axis() {
        let mut editor = document();
        let before = editor.elements()[0].clone();
        for factors in [
            Point {
                x: 2.0,
                y: f32::NAN,
            },
            Point { x: -1.0, y: 2.0 },
            Point { x: 2.0, y: 0.0 },
            Point {
                x: 2.0,
                y: f32::INFINITY,
            },
        ] {
            assert!(editor.resize_axes(0, factors).is_err());
            assert_eq!(editor.elements()[0].points, before.points);
            assert_eq!(editor.elements()[0].style(), before.style());
            assert_eq!(editor.retained_points, 2);
        }
    }
    fn document() -> Editor {
        let mut editor = Editor::default();
        // 几何缩放测试显式选用非零圆角，不依赖产品默认样式。
        editor.set_style(
            editor
                .style()
                .with_width(4.0)
                .unwrap()
                .with_corner_radius(12.0)
                .unwrap(),
        );
        editor
            .begin(Tool::RoundedRectangle, Point { x: 10.0, y: 20.0 })
            .unwrap();
        editor.extend(Point { x: 30.0, y: 60.0 }).unwrap();
        editor.commit();
        editor
    }
    #[test]
    fn resize_preserves_center_and_scales_style_with_exact_undo() {
        let mut editor = document();
        let before = editor.elements()[0].clone();
        let pointer = editor.elements()[0].points.as_ptr();
        assert!(editor.resize(0, 2.0).unwrap());
        let after = &editor.elements()[0];
        assert_eq!(
            after.points,
            vec![Point { x: 0.0, y: 0.0 }, Point { x: 40.0, y: 80.0 }]
        );
        assert_eq!(after.width, 8.0);
        assert_eq!(after.corner_radius, 24.0);
        assert_eq!(editor.retained_points, 4);
        editor.undo();
        assert_eq!(editor.elements()[0].points, before.points);
        assert_eq!(editor.elements()[0].style(), before.style());
        assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
        editor.redo();
        assert_eq!(editor.elements()[0].width, 8.0);
    }
    #[test]
    fn invalid_scale_and_resource_limits_are_transactional() {
        let mut editor = document();
        let revision = editor.revision();
        for factor in [0.0, -1.0, f32::NAN, f32::INFINITY, 100_000.0] {
            assert!(editor.resize(0, factor).is_err());
            assert_eq!(editor.revision(), revision);
            assert_eq!(editor.retained_points, 2);
        }
        assert!(editor.resize(0, 1000.0).unwrap());
        assert_eq!(editor.elements()[0].width, 4000.0);
        assert_eq!(editor.elements()[0].corner_radius, 12000.0);
        assert!(editor.undo());
        assert!(!editor.resize(0, 1.0).unwrap());
        assert!(!editor.resize(99, 2.0).unwrap());
        editor.retained_points = MAX_TOTAL_POINTS;
        assert_eq!(editor.resize(0, 2.0), Err(EditError::BudgetExceeded));
        assert_eq!(editor.elements()[0].points[0], Point { x: 10.0, y: 20.0 });
    }
}
