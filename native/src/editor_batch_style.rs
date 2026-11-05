//! 多选样式事务只保存属性；几何、文本和效果像素保持原所有权，不消耗额外点预算。
use super::*;

impl Editor {
    pub fn restyle_many(
        &mut self,
        indices: &[usize],
        style: StrokeStyle,
    ) -> Result<bool, EditError> {
        if indices.iter().any(|index| *index >= self.elements.len())
            || !indices.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EditError::InvalidPoint);
        }
        if self.draft.is_some() || indices.is_empty() {
            return Ok(false);
        }
        // 先建立完整历史候选，再写入属性；不把未变化对象加入历史。
        let previous: Vec<_> = indices
            .iter()
            .filter_map(|index| {
                let old = self.elements[*index].style();
                (old != style).then_some((*index, old))
            })
            .collect();
        if previous.is_empty() {
            return Ok(false);
        }
        for (index, _) in &previous {
            self.elements[*index].apply_style(style);
        }
        self.retained_points -= self.history.record(Entry::styled_many(previous));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::default();
        for x in [10.0, 30.0, 50.0] {
            editor.begin(Tool::Rectangle, Point { x, y: 10.0 }).unwrap();
            editor
                .extend(Point {
                    x: x + 8.0,
                    y: 18.0,
                })
                .unwrap();
            assert!(editor.commit());
        }
        editor
    }
    #[test]
    fn noncontiguous_style_changes_share_one_history_without_geometry_copies() {
        let mut editor = document();
        let before: Vec<_> = editor.elements().iter().map(Stroke::style).collect();
        let points: Vec<_> = editor
            .elements()
            .iter()
            .map(|s| s.points.as_ptr())
            .collect();
        let retained = editor.retained_points;
        let revision = editor.revision();
        let style = StrokeStyle::default()
            .with_color(Color(1, 2, 3))
            .with_width(5.0)
            .unwrap();
        assert!(editor.restyle_many(&[0, 2], style).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        assert_eq!(editor.elements()[1].style(), before[1]);
        for _ in 0..3 {
            for index in [0, 2] {
                assert_eq!(editor.elements()[index].style(), style);
            }
            assert!(editor.undo());
            assert_eq!(
                editor
                    .elements()
                    .iter()
                    .map(Stroke::style)
                    .collect::<Vec<_>>(),
                before
            );
            assert!(editor.redo());
        }
        assert_eq!(
            editor
                .elements()
                .iter()
                .map(|s| s.points.as_ptr())
                .collect::<Vec<_>>(),
            points
        );
        assert_eq!(editor.retained_points, retained);
    }
    #[test]
    fn invalid_draft_and_noop_preserve_redo_and_do_not_partially_apply() {
        let mut editor = document();
        let style = editor.style().with_color(Color(0, 1, 2));
        assert!(editor.restyle_many(&[0, 2], style).unwrap());
        editor.undo();
        let revision = editor.revision();
        for indices in [vec![0, 3], vec![0, 0], vec![2, 0]] {
            assert_eq!(
                editor.restyle_many(&indices, style),
                Err(EditError::InvalidPoint)
            );
        }
        assert!(!editor.restyle_many(&[], style).unwrap());
        assert!(!editor.restyle_many(&[0, 2], editor.style()).unwrap());
        editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(!editor.restyle_many(&[0, 2], style).unwrap());
        editor.cancel();
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
    }
}
