//! 行首/行尾使用 DirectWrite 的逻辑行范围；换行符不属于 End 选中的本行文字。
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
        // 与既有文本上限绑定：最多 65537 行，临时数组随本次键操作释放，不常驻缓存。
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
                // End 的 UTF-16 索引可能也是下一软换行行首；保存本行 y 消除归属歧义。
                return Ok((edge, [x, top + line.height / 2.]));
            }
            start = next;
            top = bottom;
        }
        Err(invalid())
    }
}
#[cfg(test)]
#[path = "surface_text_line_edge_tests.rs"]
mod tests;
