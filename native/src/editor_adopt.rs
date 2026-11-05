//! 将显示来源接管为可编辑文档的一次事务；原文档与历史保留，像素只共享不复制。
use super::*;
use crate::edit_history::Document;

#[cfg(test)]
#[path = "editor_adopt_tests.rs"]
mod tests;

impl Editor {
    pub fn adopt_snapshot(
        &mut self,
        expected_revision: u64,
        source: &Editor,
    ) -> Result<bool, EditError> {
        if self.revision != expected_revision || self.draft.is_some() || source.draft.is_some() {
            return Ok(false);
        }
        let points = source
            .elements
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>();
        if source.elements.len() > MAX_ELEMENTS
            || self
                .retained_points
                .checked_add(points)
                .is_none_or(|total| total > MAX_TOTAL_POINTS)
        {
            return Err(EditError::BudgetExceeded);
        }
        // 先克隆有界矢量，之后的所有权交换不再发生可恢复失败；不接管来源历史或草稿。
        let elements = source.elements.clone();
        let previous = Document {
            elements: std::mem::replace(&mut self.elements, elements),
            serial: std::mem::replace(&mut self.serial_count, source.serial_count),
            raster: std::mem::replace(&mut self.raster, source.raster.clone()),
            crop: std::mem::replace(&mut self.crop, source.crop),
            logical_size: std::mem::replace(&mut self.logical_size, source.logical_size),
        };
        self.retained_points += points;
        self.retained_points -= self.history.record(Entry::document(previous));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}
