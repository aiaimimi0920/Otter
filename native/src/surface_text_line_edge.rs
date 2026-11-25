//! Home/End use DirectWrite logical line ranges; newline characters are excluded from the current line's text selected by End.
use super::*;
impl Layout {
    pub fn line_edge(
        &self,
        position: u32,
        end: bool,
        preferred: Option<[f32; 2]>,
    ) -> Result<(u32, [f32; 2])> {
        self.validate_range(position..position)?;
        if preferred.is_some_and(|p| p.iter().any(|v| !v.is_finite() || v.abs() > 16_777_216.)) {
            return Err(invalid());
        }
        if self.text_len() == 0 {
            return Ok((0, [0.; 2]));
        }
        let mut x = 0.;
        let mut y = 0.;
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native
                .HitTestTextPosition(position, false, &mut x, &mut y, &mut hit)?;
        }
        let row = preferred.map_or(hit.top + hit.height / 2., |p| p[1]);
        let count = self.metrics.lineCount;
        if count == 0 || count > self.text_len().saturating_add(1) || !row.is_finite() || row < 0. {
            return Err(invalid());
        }
        // Bound to existing text limits: at most 65537 lines, with temporary arrays released after each key operation rather than cached permanently.
        let mut lines = vec![DWRITE_LINE_METRICS::default(); count as usize];
        let mut actual = 0;
        unsafe {
            self.native.GetLineMetrics(Some(&mut lines), &mut actual)?;
        }
        if actual != count {
            return Err(invalid());
        }
        let (mut start, mut top) = (0u32, 0f32);
        for (i, line) in lines.iter().enumerate() {
            let next = start
                .checked_add(line.length)
                .filter(|v| *v <= self.text_len())
                .ok_or_else(invalid)?;
            let bottom = top + line.height;
            if !bottom.is_finite() || line.height <= 0. || line.newlineLength > line.length {
                return Err(invalid());
            }
            if row < bottom || i + 1 == lines.len() {
                let edge = if end {
                    next - line.newlineLength
                } else {
                    start
                };
                self.validate_range(edge..edge)?;
                let trailing = end && edge > start;
                let probe = if trailing { edge - 1 } else { edge };
                unsafe {
                    self.native
                        .HitTestTextPosition(probe, trailing, &mut x, &mut y, &mut hit)?;
                }
                if !x.is_finite() {
                    return Err(invalid());
                }
                // End's UTF-16 index may also be the next soft-wrapped line's start; save this line's y to disambiguate ownership.
                return Ok((edge, [x, top + line.height / 2.]));
            }
            start = next;
            top = bottom;
        }
        Err(invalid())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_text_line_edge_tests.rs"]
mod tests;
