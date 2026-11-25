//! Single-node word ranges; initial double-click includes trailing whitespace, while dragging uses word segments without whitespace expansion.
use super::*;
#[path = "surface_text_word_icu.rs"]
mod icu;
impl Layout {
    pub(crate) fn word_boundaries(&self) -> Result<Vec<u32>> {
        icu::boundaries(&self.text)
    }
    pub(crate) fn word_range(&self, position: u32, boundaries: &[u32]) -> Result<Range<u32>> {
        let range = self.word_segment(position, boundaries)?;
        let start = range.start;
        let mut end = range.end;
        // Initial double-click includes horizontal trailing whitespace; this rule must not apply to subsequent word-level dragging.
        while let Some(unit) = self.text.get(end as usize) {
            if matches!(*unit, 10 | 13 | 0x2028 | 0x2029)
                || !char::from_u32(*unit as u32).is_some_and(char::is_whitespace)
            {
                break;
            }
            end += 1;
        }
        Ok(start..end)
    }
    pub(crate) fn word_segment(&self, position: u32, boundaries: &[u32]) -> Result<Range<u32>> {
        self.validate_range(position..position)?;
        if self.text.is_empty() {
            return Ok(0..0);
        }
        if let Some(range) = self.omitted_range(position)? {
            return Ok(range);
        }
        let index = boundaries.partition_point(|at| *at <= position.min(self.text_len() - 1));
        let (Some(&start), Some(&end)) = (
            index.checked_sub(1).and_then(|i| boundaries.get(i)),
            boundaries.get(index),
        ) else {
            return Err(invalid());
        };
        self.validate_range(start..end)?;
        Ok(start..end)
    }
}
