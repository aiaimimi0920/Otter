//! 贝塞尔局部控制点事务；一项操作只复制目标曲线，旧值归历史所有。
use super::*;

const MAX_SEGMENTS: usize = 64;

fn valid(stroke: &Stroke) -> bool {
    stroke.tool == Tool::Bezier
        && stroke.points.len() >= 4
        && stroke.points.len() <= MAX_SEGMENTS * 3 + 1
        && (stroke.points.len() - 1).is_multiple_of(3)
}

impl Editor {
    /// point 位于标注未旋转的局部图像坐标；宿主负责逆变换鼠标坐标。
    pub fn move_bezier_handle(
        &mut self,
        index: usize,
        handle: usize,
        point: Point,
    ) -> Result<bool, EditError> {
        validate(point)?;
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(source) = self.elements.get(index).filter(|s| valid(s)) else {
            return Ok(false);
        };
        let Some(&old) = source.points.get(handle) else {
            return Ok(false);
        };
        if old == point {
            return Ok(false);
        }
        if self.retained_points + source.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let changed = source.preview_bezier_handle(handle, point)?.unwrap();
        self.commit_bezier(index, changed);
        Ok(true)
    }
}

impl Stroke {
    /// 有界预览副本；不接触文档版本或历史，提交与屏幕预览使用同一计算。
    pub fn preview_bezier_handle(
        &self,
        handle: usize,
        point: Point,
    ) -> Result<Option<Self>, EditError> {
        validate(point)?;
        if !valid(self) {
            return Ok(None);
        }
        let Some(&old) = self.points.get(handle) else {
            return Ok(None);
        };
        let source = self;
        let offset = Point {
            x: point.x - old.x,
            y: point.y - old.y,
        };
        let translated = |p: Point| Point {
            x: p.x + offset.x,
            y: p.y + offset.y,
        };
        // 端点携带相邻切线；先验证所有目标，避免一侧合法、另一侧越界的部分提交。
        let adjacent = [
            handle.checked_sub(1),
            handle.checked_add(1).filter(|&h| h < source.points.len()),
        ];
        if handle.is_multiple_of(3) {
            for h in adjacent.into_iter().flatten() {
                validate(translated(source.points[h]))?;
            }
        }
        let mut changed = source.clone();
        changed.points[handle] = point;
        if handle.is_multiple_of(3) {
            for h in adjacent.into_iter().flatten() {
                changed.points[h] = translated(changed.points[h]);
            }
        }
        preserve_rotation_center(source, &mut changed)?;
        Ok(Some(changed))
    }
}

impl Editor {
    pub fn extend_bezier(
        &mut self,
        index: usize,
        end: Point,
        from_start: bool,
    ) -> Result<bool, EditError> {
        validate(end)?;
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(source) = self.elements.get(index).filter(|s| valid(s)) else {
            return Ok(false);
        };
        let count = source.points.len();
        if count > MAX_SEGMENTS * 3 {
            return Err(EditError::BudgetExceeded);
        }
        if self.retained_points + count + 3 > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let changed = source.preview_extend_bezier(end, from_start)?.unwrap();
        self.commit_bezier(index, changed);
        Ok(true)
    }

    fn commit_bezier(&mut self, index: usize, changed: Stroke) {
        let count = changed.points.len();
        let old = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += count;
        self.retained_points -= self.history.record(Entry::replaced(index, old));
        self.revision = self.revision.wrapping_add(1);
    }
}

impl Stroke {
    pub fn preview_extend_bezier(
        &self,
        end: Point,
        from_start: bool,
    ) -> Result<Option<Self>, EditError> {
        validate(end)?;
        if !valid(self) {
            return Ok(None);
        }
        let source = self;
        let count = source.points.len();
        if count > MAX_SEGMENTS * 3 {
            return Err(EditError::BudgetExceeded);
        }
        let (start, previous) = if from_start {
            (source.points[0], source.points[1])
        } else {
            (source.points[count - 1], source.points[count - 2])
        };
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length = dx.hypot(dy) / 3.0;
        let tangent = (start.x - previous.x).hypot(start.y - previous.y);
        let c1 = if tangent > 0.001 {
            Point {
                x: start.x + (start.x - previous.x) / tangent * length,
                y: start.y + (start.y - previous.y) / tangent * length,
            }
        } else {
            Point {
                x: start.x + dx / 3.0,
                y: start.y + dy / 3.0,
            }
        };
        let c2 = Point {
            x: end.x - dx / 3.0,
            y: end.y - dy / 3.0,
        };
        validate(c1)?;
        validate(c2)?;
        let mut changed = source.clone();
        if from_start {
            changed.points.splice(0..0, [end, c2, c1]);
        } else {
            changed.points.extend_from_slice(&[c1, c2, end]);
        }
        preserve_rotation_center(source, &mut changed)?;
        Ok(Some(changed))
    }
}

fn preserve_rotation_center(source: &Stroke, changed: &mut Stroke) -> Result<(), EditError> {
    // 控制点变化会移动包围中心；补偿旋转轴变化，保持旧点的屏幕位置。
    crate::rotation::preserve_center(&source.points, &mut changed.points, source.rotation);
    for &point in &changed.points {
        validate(point)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn total_budget_rejects_handle_or_extension_before_mutation() {
        let mut editor = document();
        let before = editor.elements()[0].points.clone();
        let revision = editor.revision();
        editor.retained_points = MAX_TOTAL_POINTS - 3;
        assert_eq!(
            editor.move_bezier_handle(0, 1, p(30.0, 40.0)),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(
            editor.extend_bezier(0, p(100.0, 40.0), false),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(editor.elements()[0].points, before);
        assert_eq!(editor.revision(), revision);
    }
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    fn document() -> Editor {
        let mut editor = Editor::default();
        editor.begin(Tool::Bezier, p(10.0, 10.0)).unwrap();
        editor.extend(p(70.0, 10.0)).unwrap();
        editor.commit();
        editor
    }
    #[test]
    fn endpoint_carries_tangent_and_undo_restores_exact_storage() {
        let mut editor = document();
        let before = editor.elements()[0].clone();
        let pointer = editor.elements()[0].points.as_ptr();
        editor.move_bezier_handle(0, 0, p(20.0, 30.0)).unwrap();
        assert_eq!(editor.elements()[0].points[1], p(40.0, 45.0));
        assert_eq!(editor.elements()[0].points[2], before.points[2]);
        assert_eq!(editor.retained_points, 8);
        editor.undo();
        assert_eq!(editor.elements()[0].points, before.points);
        assert_eq!(editor.elements()[0].points.as_ptr(), pointer);
        editor.redo();
        editor.move_bezier_handle(0, 1, p(15.0, 40.0)).unwrap();
        assert_eq!(editor.elements()[0].points[0], p(20.0, 30.0));
    }
    #[test]
    fn extension_preserves_old_controls_and_internal_node_carries_both_tangents() {
        let mut editor = document();
        let before = editor.elements()[0].points.clone();
        editor.extend_bezier(0, p(100.0, 40.0), false).unwrap();
        assert_eq!(&editor.elements()[0].points[..4], &before);
        assert_eq!(editor.elements()[0].points.len(), 7);
        assert_eq!(editor.retained_points, 11);
        let extended = editor.elements()[0].points.clone();
        editor.move_bezier_handle(0, 3, p(75.0, 15.0)).unwrap();
        for index in [2, 4] {
            assert_eq!(
                editor.elements()[0].points[index],
                p(extended[index].x + 5.0, extended[index].y + 5.0)
            );
        }
        editor.undo();
        editor.undo();
        assert_eq!(editor.elements()[0].points, before);
        editor.extend_bezier(0, p(-20.0, 40.0), true).unwrap();
        assert_eq!(&editor.elements()[0].points[3..], &before);
        assert_eq!(editor.elements()[0].points[0], p(-20.0, 40.0));
    }
    #[test]
    fn failures_and_noops_keep_redo_and_segment_limit_is_bounded() {
        let mut editor = document();
        editor.move_bezier_handle(0, 1, p(15.0, 15.0)).unwrap();
        editor.undo();
        let revision = editor.revision();
        assert!(!editor.move_bezier_handle(0, 99, p(0.0, 0.0)).unwrap());
        assert!(!editor.move_bezier_handle(0, 0, p(10.0, 10.0)).unwrap());
        assert_eq!(
            editor.move_bezier_handle(0, 1, p(f32::NAN, 0.0)),
            Err(EditError::InvalidPoint)
        );
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
        for n in 1..64 {
            assert!(editor
                .extend_bezier(0, p(70.0 + n as f32 * 5.0, 10.0), false)
                .unwrap());
        }
        assert_eq!(editor.elements()[0].points.len(), 193);
        let revision = editor.revision();
        assert_eq!(
            editor.extend_bezier(0, p(500.0, 10.0), false),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(editor.revision(), revision);
    }
}
