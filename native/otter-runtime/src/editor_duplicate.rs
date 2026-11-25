//! 复制沿用基线 12px 偏移和追加到顶层语义；原标注和已生成导出快照不变。
use super::*;

impl Editor {
    pub fn duplicate(&mut self, index: usize) -> Result<Option<usize>, EditError> {
        if self.draft.is_some() {
            return Ok(None);
        }
        let Some(source) = self.elements.get(index) else {
            return Ok(None);
        };
        if self.elements.len() >= MAX_ELEMENTS
            || self.retained_points + source.points.len() > MAX_TOTAL_POINTS
        {
            return Err(EditError::BudgetExceeded);
        }
        for point in &source.points {
            validate(Point {
                x: point.x + 12.0,
                y: point.y + 12.0,
            })?;
        }
        let mut duplicated = source.clone();
        duplicated.annotation_id = annotation_id::AnnotationId::new()?;
        for point in &mut duplicated.points {
            point.x += 12.0;
            point.y += 12.0;
        }
        let index = self.elements.len();
        self.retained_points += duplicated.points.len();
        self.elements.push(duplicated);
        self.retained_points -= self.history.record(Entry::added(index));
        self.revision = self.revision.wrapping_add(1);
        Ok(Some(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::default();
        editor
            .begin(Tool::Rectangle, Point { x: 10.0, y: 20.0 })
            .unwrap();
        editor.extend(Point { x: 30.0, y: 60.0 }).unwrap();
        editor.commit();
        editor.rotate(0, 45.0).unwrap();
        editor
    }
    #[test]
    fn duplicate_preserves_style_rotation_and_independent_points() {
        let mut editor = document();
        let source = editor.elements()[0].clone();
        let snapshot = editor.export_snapshot();
        assert_eq!(editor.duplicate(0).unwrap(), Some(1));
        let copy = &editor.elements()[1];
        assert_eq!(
            copy.points,
            vec![Point { x: 22.0, y: 32.0 }, Point { x: 42.0, y: 72.0 }]
        );
        assert_eq!(copy.style(), source.style());
        assert_eq!(copy.rotation, source.rotation);
        assert_ne!(copy.points.as_ptr(), editor.elements()[0].points.as_ptr());
        assert_eq!(editor.retained_points, 4);
        editor.translate(1, Point { x: 5.0, y: 0.0 }).unwrap();
        assert_eq!(editor.elements()[0].points, source.points);
        assert_eq!(snapshot.elements().len(), 1);
        editor.undo();
        editor.undo();
        assert_eq!(editor.elements().len(), 1);
        editor.redo();
        assert_eq!(editor.elements().len(), 2);
        assert_eq!(editor.elements()[1].rotation, 45.0);
    }
    #[test]
    fn invalid_index_and_budget_failure_do_not_discard_redo() {
        let mut editor = document();
        editor.duplicate(0).unwrap();
        editor.undo();
        let revision = editor.revision();
        assert_eq!(editor.duplicate(99).unwrap(), None);
        let count = editor.retained_points;
        editor.retained_points = MAX_TOTAL_POINTS;
        assert_eq!(editor.duplicate(0), Err(EditError::BudgetExceeded));
        editor.retained_points = count;
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
    }
}
