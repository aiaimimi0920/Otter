//! 多对象事务按严格递增索引一次提交；先验证整个集合，历史持有原对象以精确恢复。
use super::*;

impl Editor {
    /// 组移动先检查全部坐标与共享点预算，失败不改变文档或重做分支。
    pub fn translate_many(&mut self, indices: &[usize], offset: Point) -> Result<bool, EditError> {
        validate(offset)?;
        if indices.iter().any(|index| *index >= self.elements.len())
            || !indices.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EditError::InvalidPoint);
        }
        if indices.is_empty() || self.draft.is_some() || offset == Point::default() {
            return Ok(false);
        }
        let extra: usize = indices
            .iter()
            .map(|index| self.elements[*index].points.len())
            .sum();
        if self.retained_points + extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        for index in indices {
            for point in &self.elements[*index].points {
                validate(Point {
                    x: point.x + offset.x,
                    y: point.y + offset.y,
                })?;
            }
        }
        // 所有可恢复错误在此之前返回；候选完整建立后才开始交换文档对象。
        let mut replacements: Vec<_> = indices
            .iter()
            .map(|index| {
                let mut moved = self.elements[*index].clone();
                for point in &mut moved.points {
                    point.x += offset.x;
                    point.y += offset.y;
                }
                (*index, moved)
            })
            .collect();
        for (index, moved) in &mut replacements {
            std::mem::swap(moved, &mut self.elements[*index]);
        }
        self.retained_points += extra;
        self.retained_points -= self.history.record(Entry::replaced_many(replacements));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
    pub fn remove_many(&mut self, indices: &[usize]) -> Result<bool, EditError> {
        if indices.is_empty() {
            return Ok(false);
        }
        if indices.iter().any(|index| *index >= self.elements.len())
            || !indices.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EditError::InvalidPoint);
        }
        if self.draft.is_some() {
            return Ok(false);
        }
        let mut removed = Vec::with_capacity(indices.len());
        for index in indices.iter().rev() {
            removed.push((*index, Some(self.elements.remove(*index))));
        }
        removed.reverse();
        self.retained_points -= self.history.record(Entry::removed_many(removed));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> Editor {
        let mut editor = Editor::default();
        for x in 0..6 {
            editor
                .begin(
                    Tool::Brush,
                    Point {
                        x: x as f32,
                        y: 0.0,
                    },
                )
                .unwrap();
            editor
                .extend(Point {
                    x: x as f32,
                    y: 4.0,
                })
                .unwrap();
            assert!(editor.commit());
        }
        editor
    }
    #[test]
    fn noncontiguous_batch_has_one_revision_and_restores_all_allocations_in_order() {
        let mut editor = editor();
        let pointers: Vec<_> = editor.elements.iter().map(|s| s.points.as_ptr()).collect();
        let revision = editor.revision();
        let points = editor.retained_points;
        assert!(editor.remove_many(&[0, 2, 5]).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(
            editor
                .elements
                .iter()
                .map(|s| s.points[0].x)
                .collect::<Vec<_>>(),
            vec![1.0, 3.0, 4.0]
        );
        assert_eq!(editor.retained_points, points);
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
            assert_eq!(editor.elements.len(), 3);
            assert_eq!(editor.retained_points, points);
        }
    }
    #[test]
    fn invalid_or_draft_batch_never_partially_removes_or_drops_redo() {
        let mut editor = editor();
        assert!(editor.undo());
        let revision = editor.revision();
        for indices in [&[0, 0][..], &[4, 0], &[0, 5]] {
            assert!(editor.remove_many(indices).is_err());
        }
        assert!(!editor.remove_many(&[]).unwrap());
        editor.begin(Tool::Brush, Point { x: 0.0, y: 0.0 }).unwrap();
        assert!(!editor.remove_many(&[0, 4]).unwrap());
        editor.cancel();
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
        assert_eq!(editor.elements.len(), 6);
    }
    #[test]
    fn evicting_applied_batch_releases_its_retained_points_only() {
        let mut editor = editor();
        assert!(editor.remove_many(&[0, 1, 2, 3, 4, 5]).unwrap());
        assert_eq!(editor.retained_points, 12);
        for x in 0..50 {
            editor
                .begin(
                    Tool::Brush,
                    Point {
                        x: x as f32,
                        y: 0.0,
                    },
                )
                .unwrap();
            assert!(editor.commit());
        }
        assert_eq!(editor.retained_points, 50);
        assert_eq!(editor.elements.len(), 50);
        for _ in 0..50 {
            assert!(editor.undo());
        }
        assert!(!editor.undo());
        assert!(editor.elements.is_empty());
    }
    #[test]
    fn group_move_is_one_revision_with_exact_undo_and_stable_allocations() {
        let mut editor = editor();
        let pointers: Vec<_> = editor.elements.iter().map(|s| s.points.as_ptr()).collect();
        let revision = editor.revision();
        assert!(editor
            .translate_many(&[0, 2, 5], Point { x: 0.1, y: -2.5 })
            .unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.retained_points, 18);
        assert_eq!(editor.elements[1].points.as_ptr(), pointers[1]);
        let moved: Vec<_> = editor.elements.iter().map(|s| s.points.as_ptr()).collect();
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
            assert_eq!(editor.elements[0].points[0], Point { x: 0.0, y: 0.0 });
            assert!(editor.redo());
            assert_eq!(
                editor
                    .elements
                    .iter()
                    .map(|s| s.points.as_ptr())
                    .collect::<Vec<_>>(),
                moved
            );
        }
    }
    #[test]
    fn invalid_group_move_keeps_every_object_and_redo() {
        let mut editor = editor();
        assert!(editor.undo());
        let revision = editor.revision();
        for indices in [&[0, 0][..], &[2, 0], &[0, 5]] {
            assert!(editor
                .translate_many(indices, Point { x: 1.0, y: 0.0 })
                .is_err());
        }
        assert!(editor
            .translate_many(
                &[0, 4],
                Point {
                    x: 1_000_000.0,
                    y: 0.0
                }
            )
            .is_err());
        assert_eq!(editor.elements[0].points[0].x, 0.0);
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
        editor.retained_points = MAX_TOTAL_POINTS;
        assert!(matches!(
            editor.translate_many(&[0, 5], Point { x: 1.0, y: 0.0 }),
            Err(EditError::BudgetExceeded)
        ));
        assert_eq!(editor.elements[0].points[0].x, 0.0);
    }
    #[test]
    fn replacing_redo_branch_releases_group_move_snapshots() {
        let mut editor = editor();
        editor
            .translate_many(&[0, 2, 5], Point { x: 1.0, y: 0.0 })
            .unwrap();
        assert_eq!(editor.retained_points, 18);
        assert!(editor.undo());
        editor
            .translate_many(&[1], Point { x: 2.0, y: 0.0 })
            .unwrap();
        assert_eq!(editor.retained_points, 14);
        assert!(!editor.redo());
    }
}
