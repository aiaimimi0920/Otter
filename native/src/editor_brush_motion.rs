//! 输入时序到笔刷权重的草稿适配；只在点实际接受后前进时序，提交不保存时间。
use super::*;

impl Editor {
    pub fn begin_timed(&mut self, tool: Tool, point: Point, time: f64) -> Result<bool, EditError> {
        let started = self.begin(tool, point)?;
        if started && tool == Tool::Paintbrush {
            self.draft_motion = Some(crate::brush_dynamics::sample(point, time, None));
        }
        Ok(started)
    }

    /// raw_point 保留吸附前坐标，避免 Shift 角度锁定改变速度估计。
    pub fn extend_timed(
        &mut self,
        point: Point,
        straight: bool,
        time: f64,
        raw_point: Point,
    ) -> Result<bool, EditError> {
        validate(raw_point)?;
        let motion = self
            .draft
            .as_ref()
            .filter(|s| s.tool == Tool::Paintbrush)
            .map(|_| crate::brush_dynamics::sample(raw_point, time, self.draft_motion));
        let changed = self.extend_constrained(point, straight)?;
        if changed {
            if let Some(motion) = motion {
                let draft = self.draft.as_mut().unwrap();
                *draft.brush_weights.last_mut().unwrap() = motion.weight;
                self.draft_motion = Some(motion);
            }
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_sample_does_not_advance_clock_or_weights() {
        let mut editor = Editor::default();
        editor
            .begin_timed(Tool::Paintbrush, Point::default(), 100.0)
            .unwrap();
        editor.retained_points = MAX_TOTAL_POINTS - 1;
        let point = Point { x: 10.0, y: 0.0 };
        assert_eq!(
            editor.extend_timed(point, false, 104.0, point),
            Err(EditError::BudgetExceeded)
        );
        assert_eq!(editor.draft_motion.unwrap().time, 100.0);
        assert_eq!(editor.draft().unwrap().brush_weights, vec![0.85]);
        assert_eq!(editor.draft().unwrap().points.len(), 1);
    }
    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }
    #[test]
    fn accepted_motion_weights_survive_history_transform_and_snapshot() {
        let mut editor = Editor::default();
        editor
            .begin_timed(Tool::Paintbrush, p(0.0, 0.0), 100.0)
            .unwrap();
        assert!(!editor
            .extend_timed(p(0.2, 0.0), false, 101.0, p(0.2, 0.0))
            .unwrap());
        editor
            .extend_timed(p(30.0, 0.0), false, 104.0, p(30.0, 0.0))
            .unwrap();
        let expected = crate::brush_dynamics::sample(
            p(30.0, 0.0),
            104.0,
            Some(crate::brush_dynamics::sample(p(0.0, 0.0), 100.0, None)),
        )
        .weight;
        assert_eq!(editor.draft().unwrap().brush_weights, vec![0.85, expected]);
        editor.commit_smoothed(50.0);
        assert!(editor.draft_motion.is_none());
        let weights = editor.elements()[0].brush_weights.clone();
        let snapshot = editor.export_snapshot();
        editor.resize(0, 2.0).unwrap();
        assert_eq!(editor.elements()[0].brush_weights, weights);
        editor.undo();
        editor.undo();
        editor.redo();
        assert_eq!(editor.elements()[0].brush_weights, weights);
        editor.duplicate(0).unwrap();
        assert_eq!(editor.elements()[1].brush_weights, weights);
        assert_eq!(snapshot.elements()[0].brush_weights, weights);
    }
    #[test]
    fn straight_mode_keeps_weights_aligned_and_cancel_clears_clock() {
        let mut editor = Editor::default();
        editor
            .begin_timed(Tool::Paintbrush, p(0.0, 0.0), 0.0)
            .unwrap();
        for i in 1..5 {
            let point = p(i as f32 * 10.0, 5.0);
            editor
                .extend_timed(point, false, i as f64 * 16.0, point)
                .unwrap();
        }
        editor
            .extend_timed(p(50.0, 0.0), true, 80.0, p(50.0, 5.0))
            .unwrap();
        assert_eq!(editor.draft().unwrap().points.len(), 2);
        assert_eq!(editor.draft().unwrap().brush_weights.len(), 2);
        editor
            .extend_timed(p(60.0, 10.0), false, 96.0, p(60.0, 10.0))
            .unwrap();
        assert_eq!(editor.draft().unwrap().brush_weights.len(), 3);
        editor.cancel();
        assert!(editor.draft_motion.is_none());
    }
}
