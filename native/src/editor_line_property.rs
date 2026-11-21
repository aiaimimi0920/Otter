//! 已选直线的箭头开关需包含工具类型换代；以原有单个替换历史保存完整原貌。
use super::*;
impl Editor {
    pub(crate) fn set_line_arrow(
        &mut self,
        index: usize,
        enabled: bool,
    ) -> Result<bool, EditError> {
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(source) = self.elements.get(index) else {
            return Ok(false);
        };
        if !matches!(source.tool, Tool::Line | Tool::Arrow | Tool::Bezier) {
            return Ok(false);
        }
        let active = source.tool == Tool::Arrow || source.arrow;
        if active == enabled {
            return Ok(false);
        }
        if self.retained_points + source.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let mut changed = source.clone();
        if changed.tool == Tool::Arrow {
            changed.tool = Tool::Line;
        }
        changed.arrow = enabled;
        let old = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += old.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, old));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}
