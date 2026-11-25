//! Text-model transactions for multiline import; prepare all lines first, record history once on success, and do not clone the existing document.
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
        // Match the clipboard multiline limit; callers must not bypass document/history-point budgets through the batch API.
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
        // Geometry, quotas, and ID allocation have all succeeded; do not change elements, redo, or revision before this point.
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
