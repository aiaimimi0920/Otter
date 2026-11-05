//! 序号创建与计数器是同一次历史事务；删除/复制不重排已有标签。
use super::*;
use crate::text_annotation::TextAnnotation;
impl Editor {
    pub fn serial_count(&self) -> u32 {
        self.serial_count
    }
    pub fn insert_serial(
        &mut self,
        center: Point,
        radius: f32,
        family: &str,
    ) -> Result<Option<usize>, EditError> {
        self.insert_serial_with_fill(center, radius, family, Some(Color(0, 0, 0)))
    }
    pub fn insert_serial_with_fill(
        &mut self,
        center: Point,
        radius: f32,
        family: &str,
        fill: Option<Color>,
    ) -> Result<Option<usize>, EditError> {
        validate(center)?;
        if !radius.is_finite() {
            return Err(EditError::InvalidPoint);
        }
        let radius = radius.round().clamp(8.0, 999.0);
        let start = Point {
            x: center.x - radius,
            y: center.y - radius,
        };
        let end = Point {
            x: center.x + radius,
            y: center.y + radius,
        };
        validate(start)?;
        validate(end)?;
        if self.draft.is_some() {
            return Ok(None);
        }
        if self.elements.len() >= MAX_ELEMENTS || self.retained_points + 2 > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let number = self
            .serial_count
            .checked_add(1)
            .ok_or(EditError::BudgetExceeded)?;
        let text = TextAnnotation::new(
            &number.to_string(),
            family,
            (radius * 1.15).round().max(10.0),
            false,
        )
        .ok_or(EditError::InvalidPoint)?;
        let style = self.style;
        let stroke = Stroke {
            annotation_id: annotation_id::AnnotationId::new()?,
            tool: Tool::Serial,
            points: vec![start, end],
            width: (radius / 7.0).round().max(1.0),
            color: style.color(),
            opacity: 1.0,
            fill,
            paint_alpha: crate::stroke_style::PaintAlpha::new(style.paint_alpha().stroke(), 1.0)
                .expect("validated style alpha"),
            corner_radius: 0.0,
            dash: crate::stroke_style::DashPattern::Solid,
            sides: style.sides(),
            rotation: 0.0,
            arrow: false,
            brush_weights: Vec::new(),
            text: Some(text),
            blur: None,
            mosaic_cell: style.mosaic_cell(),
        };
        let index = self.elements.len();
        self.elements.push(stroke);
        self.retained_points += 2;
        self.retained_points -= self
            .history
            .record(Entry::serial_added(index, self.serial_count));
        self.serial_count = number;
        self.revision = self.revision.wrapping_add(1);
        Ok(Some(index))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn add(editor: &mut Editor) -> usize {
        editor
            .insert_serial(Point { x: 40.0, y: 40.0 }, 14.0, "微软雅黑")
            .unwrap()
            .unwrap()
    }
    fn label(editor: &Editor, index: usize) -> &str {
        editor.elements()[index].text.as_ref().unwrap().content()
    }
    #[test]
    fn counter_follows_undo_redo_and_branch_but_delete_does_not_renumber() {
        let mut editor = Editor::default();
        add(&mut editor);
        add(&mut editor);
        assert_eq!(label(&editor, 0), "1");
        assert_eq!(label(&editor, 1), "2");
        editor.undo();
        assert_eq!(editor.serial_count, 1);
        editor.redo();
        assert_eq!(editor.serial_count, 2);
        editor.undo();
        add(&mut editor);
        assert_eq!(label(&editor, 1), "2");
        assert!(!editor.redo());
        editor.remove(0);
        let next = add(&mut editor);
        assert_eq!(label(&editor, next), "3");
        let snapshot = editor.export_snapshot();
        assert_eq!(snapshot.serial_count, 3);
        editor.duplicate(0).unwrap();
        assert_eq!(editor.serial_count, 3);
    }
    #[test]
    fn editing_serial_text_and_font_preserves_circle_and_does_not_consume_number() {
        let mut editor = Editor::default();
        add(&mut editor);
        editor.rotate(0, 45.0).unwrap();
        let before = editor.elements()[0].clone();
        let changed = TextAnnotation::new("A", "Segoe UI", 24.0, true).unwrap();
        assert!(editor
            .replace_text(0, Point { x: 500.0, y: 200.0 }, changed)
            .unwrap());
        assert_eq!(editor.elements()[0].points, before.points);
        assert_eq!(editor.elements()[0].style(), before.style());
        assert_eq!(editor.elements()[0].rotation, 45.0);
        assert_eq!(editor.serial_count, 1);
        editor.undo();
        assert_eq!(label(&editor, 0), "1");
        editor.redo();
        assert_eq!(label(&editor, 0), "A");
        let next = add(&mut editor);
        assert_eq!(label(&editor, next), "2");
    }
    #[test]
    fn invalid_input_and_budget_do_not_consume_number() {
        let mut editor = Editor::default();
        assert!(editor
            .insert_serial(Point::default(), f32::NAN, "微软雅黑")
            .is_err());
        assert!(editor.insert_serial(Point::default(), 14.0, "").is_err());
        assert_eq!(editor.serial_count, 0);
        editor.retained_points = MAX_TOTAL_POINTS;
        assert_eq!(
            editor.insert_serial(Point::default(), 14.0, "微软雅黑"),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(editor.serial_count, 0);
        assert_eq!(editor.revision(), 0);
    }
}
