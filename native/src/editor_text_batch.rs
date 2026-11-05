//! 多行导入的文字模型事务；所有行先准备，成功时只记录一次历史且不克隆已有文档。
use super::*;

impl Editor {
    pub fn insert_text_batch(
        &mut self,
        entries: &[(Point, Point, TextAnnotation)],
    ) -> Result<std::ops::Range<usize>, EditError> {
        let first = self.elements.len();
        if entries.is_empty() || self.draft.is_some() {
            return Ok(first..first);
        }
        // 对齐剪贴板多行的上界；不让调用者借批量接口绕过文档/历史点预算。
        if entries.len() > 256
            || entries.len() > MAX_ELEMENTS.saturating_sub(first)
            || entries.len() * 2 > MAX_TOTAL_POINTS.saturating_sub(self.retained_points)
        {
            return Err(EditError::BudgetExceeded);
        }
        let mut prepared = Vec::with_capacity(entries.len());
        for (start, size, text) in entries {
            let end = super::end(*start, *size)?;
            prepared.push(super::prepare(*start, end, text.clone(), self.style)?);
        }
        // 几何、配额和 ID 分配已全部成功；在此之前不改变元素、redo 或 revision。
        let indices = first..first + prepared.len();
        self.retained_points += prepared.len() * 2;
        self.elements.extend(prepared);
        self.retained_points -= self.history.record(Entry::added_many(indices.clone()));
        self.revision = self.revision.wrapping_add(1);
        Ok(indices)
    }
}

#[cfg(test)]
#[path = "editor_text_batch_tests.rs"]
mod tests;
