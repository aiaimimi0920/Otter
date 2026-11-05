//! 文字编辑事务；尺寸由原生排版层提供，不在模型中猜测字体度量。
use super::*;
use crate::text_annotation::TextAnnotation;
#[path = "editor_text_batch.rs"]
mod batch;

fn end(start: Point, size: Point) -> Result<Point, EditError> {
    validate(start)?;
    if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
        return Err(EditError::InvalidPoint);
    }
    let end = Point {
        x: start.x + size.x,
        y: start.y + size.y,
    };
    validate(end)?;
    Ok(end)
}
fn prepare(
    start: Point,
    end: Point,
    text: TextAnnotation,
    style: StrokeStyle,
) -> Result<Stroke, EditError> {
    Ok(Stroke {
        annotation_id: annotation_id::AnnotationId::new()?,
        tool: Tool::Text,
        points: vec![start, end],
        width: style.width(),
        color: style.color(),
        opacity: style.opacity(),
        fill: style.fill(),
        paint_alpha: style.paint_alpha(),
        corner_radius: style.corner_radius(),
        dash: style.dash(),
        sides: style.sides(),
        rotation: 0.0,
        arrow: false,
        brush_weights: Vec::new(),
        text: Some(text),
        blur: None,
        mosaic_cell: style.mosaic_cell(),
    })
}
impl Editor {
    pub fn insert_text(
        &mut self,
        start: Point,
        size: Point,
        text: TextAnnotation,
    ) -> Result<Option<usize>, EditError> {
        let end = end(start, size)?;
        if self.draft.is_some() {
            return Ok(None);
        }
        if self.elements.len() >= MAX_ELEMENTS || self.retained_points + 2 > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let stroke = prepare(start, end, text, self.style)?;
        let index = self.elements.len();
        self.elements.push(stroke);
        self.retained_points += 2;
        self.retained_points -= self.history.record(Entry::added(index));
        self.revision = self.revision.wrapping_add(1);
        Ok(Some(index))
    }
    pub fn replace_text(
        &mut self,
        index: usize,
        size: Point,
        text: TextAnnotation,
    ) -> Result<bool, EditError> {
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(source) = self.elements.get(index) else {
            return Ok(false);
        };
        if !matches!(source.tool, Tool::Text | Tool::Serial) || source.points.len() != 2 {
            return Ok(false);
        }
        // 序号文字与圆框尺寸分离；编辑显示内容不能把圆框变成文字布局矩形。
        let end = if source.tool == Tool::Serial {
            source.points[1]
        } else {
            end(source.points[0], size)?
        };
        if source.text.as_ref() == Some(&text) && source.points[1] == end {
            return Ok(false);
        }
        if self.retained_points + 2 > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let mut changed = source.clone();
        changed.points[1] = end;
        changed.text = Some(text);
        crate::rotation::preserve_center(&source.points, &mut changed.points, source.rotation);
        for &point in &changed.points {
            validate(point)?;
        }
        let old = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += 2;
        self.retained_points -= self.history.record(Entry::replaced(index, old));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn text(value: &str) -> TextAnnotation {
        TextAnnotation::new(value, "微软雅黑", 16.0, false).unwrap()
    }
    #[test]
    fn text_edits_history_duplicate_and_snapshot_preserve_independent_content() {
        let mut editor = Editor::default();
        let size = Point { x: 80.0, y: 24.0 };
        assert_eq!(
            editor.insert_text(Point { x: 10.0, y: 20.0 }, size, text("你好")),
            Ok(Some(0))
        );
        let snapshot = editor.export_snapshot();
        editor.duplicate(0).unwrap();
        editor.replace_text(0, size, text("新内容")).unwrap();
        assert_eq!(
            editor.elements()[1].text.as_ref().unwrap().content(),
            "你好"
        );
        assert_eq!(
            snapshot.elements()[0].text.as_ref().unwrap().content(),
            "你好"
        );
        editor.undo();
        assert_eq!(
            editor.elements()[0].text.as_ref().unwrap().content(),
            "你好"
        );
        editor.redo();
        assert_eq!(
            editor.elements()[0].text.as_ref().unwrap().content(),
            "新内容"
        );
        let revision = editor.revision();
        assert!(!editor.replace_text(0, size, text("新内容")).unwrap());
        assert_eq!(editor.revision(), revision);
        editor.retained_points = MAX_TOTAL_POINTS;
        assert_eq!(
            editor.replace_text(0, size, text("拒绝")),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(editor.revision(), revision);
    }
}
