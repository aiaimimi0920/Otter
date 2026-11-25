//! Selection hits/geometry reuse actual DirectWrite layouts without owning HWNDs, clipboards, or gesture state.
use super::*;
use std::ops::Range;
use windows::{core::BOOL, Win32::Foundation::ERROR_INSUFFICIENT_BUFFER};
#[path = "surface_text_line_edge.rs"]
mod line_edge;
#[path = "surface_text_vertical.rs"]
mod vertical;
#[path = "surface_text_word.rs"]
mod word;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextHit {
    /// UTF-16 insertion positions in normalized text, determined by full glyph-cluster starts/ends.
    pub position: u32,
    pub inside: bool,
}
fn cluster_bounds(hit: &DWRITE_HIT_TEST_METRICS) -> Result<crate::surface_clip::Rect> {
    let rect = crate::surface_clip::Rect {
        left: hit.left,
        top: hit.top,
        right: hit.left + hit.width.max(1.),
        bottom: hit.top + hit.height,
    };
    if [rect.left, rect.top, rect.right, rect.bottom]
        .iter()
        .any(|v| !v.is_finite())
        || hit.width < 0.
        || hit.height <= 0.
        || rect.empty()
    {
        return Err(invalid());
    }
    Ok(rect)
}

impl Layout {
    pub fn text_len(&self) -> u32 {
        self.text.len() as u32
    }
    /// Query the complete cluster containing this UTF-16 unit; probing surrogate tails is allowed, but split ranges are never returned.
    pub(crate) fn cluster_rect(&self, index: u32) -> Result<crate::surface_clip::Rect> {
        if index >= self.text_len() {
            return Err(invalid());
        }
        let (mut x, mut y) = (0., 0.);
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native
                .HitTestTextPosition(index, false, &mut x, &mut y, &mut hit)?;
        }
        cluster_bounds(&hit)
    }

    /// Move in logical text order by actual layout clusters, not UTF-16 units mistaken for complete characters.
    pub fn step_text(&self, position: u32, forward: bool) -> Result<u32> {
        self.validate_range(position..position)?;
        if (forward && position == self.text_len()) || (!forward && position == 0) {
            return Ok(position);
        }
        let probe = if forward { position } else { position - 1 };
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        let (mut x, mut y) = (0., 0.);
        unsafe {
            self.native
                .HitTestTextPosition(probe, false, &mut x, &mut y, &mut hit)?;
        }
        let end = hit
            .textPosition
            .checked_add(hit.length)
            .ok_or_else(invalid)?;
        if hit.length == 0 || hit.textPosition > probe || end <= probe || end > self.text_len() {
            return Err(invalid());
        }
        let next = if forward { end } else { hit.textPosition };
        self.validate_range(next..next)?;
        Ok(next)
    }

    /// Coordinates are DIPs relative to the text layout's top-left; out-of-bounds input returns the nearest edge rather than rejecting drag selection.
    pub fn hit_text(&self, point: [f32; 2]) -> Result<TextHit> {
        if point
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 16_777_216.)
        {
            return Err(invalid());
        }
        let mut trailing = BOOL::default();
        let mut inside = BOOL::default();
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native
                .HitTestPoint(point[0], point[1], &mut trailing, &mut inside, &mut hit)?;
        }
        self.normalize_trimmed_hit(&mut hit)?;
        let position = hit
            .textPosition
            .checked_add(if trailing.as_bool() { hit.length } else { 0 })
            .filter(|position| *position <= self.text_len())
            .ok_or_else(invalid)?;
        self.validate_range(position..position)?;
        Ok(TextHit {
            position,
            inside: inside.as_bool()
                || (hit.isTrimmed.as_bool()
                    && point[0] >= hit.left
                    && point[0] < hit.left + hit.width
                    && point[1] >= hit.top
                    && point[1] < hit.top + hit.height),
        })
    }

    pub(crate) fn validate_range(&self, range: Range<u32>) -> Result<()> {
        if range.start > range.end || range.end > self.text_len() {
            return Err(invalid());
        }
        // Externally supplied ranges must not split surrogate pairs either; hit entries already align to clusters.
        for position in [range.start as usize, range.end as usize] {
            if position > 0
                && position < self.text.len()
                && (0xD800..=0xDBFF).contains(&self.text[position - 1])
                && (0xDC00..=0xDFFF).contains(&self.text[position])
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    /// Construct UTF-8 only when copying; pointer movement never copies entire text blocks.
    pub fn selected_text(&self, range: Range<u32>) -> Result<String> {
        self.validate_range(range.clone())?;
        String::from_utf16(&self.text[range.start as usize..range.end as usize])
            .map_err(|_| invalid())
    }

    /// Return layout-local rectangles preserving line breaks and bidi segments; callers still apply TextItem ancestor clipping.
    pub fn selection_rects(&self, range: Range<u32>) -> Result<Vec<crate::surface_clip::Rect>> {
        self.validate_range(range.clone())?;
        if range.is_empty() {
            return Ok(Vec::new());
        }
        let mut count = 0;
        let probe = unsafe {
            self.native.HitTestTextRange(
                range.start,
                range.end - range.start,
                0.,
                0.,
                None,
                &mut count,
            )
        };
        if let Err(error) = probe {
            if error.code() != windows::core::HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0) {
                return Err(error);
            }
        }
        // Text has at most 65536 UTF-16 units; never allocate unbounded geometry buffers based on untrusted output.
        if count == 0 || count > self.text_len().saturating_add(1) {
            return Err(invalid());
        }
        let mut metrics = vec![DWRITE_HIT_TEST_METRICS::default(); count as usize];
        unsafe {
            self.native.HitTestTextRange(
                range.start,
                range.end - range.start,
                0.,
                0.,
                Some(&mut metrics),
                &mut count,
            )?;
        }
        if count as usize > metrics.len() {
            return Err(invalid());
        }
        metrics.truncate(count as usize);
        metrics
            .into_iter()
            .map(|mut hit| {
                self.normalize_trimmed_hit(&mut hit)?;
                let rect = crate::surface_clip::Rect {
                    left: hit.left,
                    top: hit.top,
                    right: hit.left + hit.width,
                    bottom: hit.top + hit.height,
                };
                if [rect.left, rect.top, rect.right, rect.bottom]
                    .iter()
                    .any(|v| !v.is_finite())
                    || hit.width < 0.
                    || hit.height < 0.
                {
                    return Err(invalid());
                }
                Ok(rect)
            })
            .collect()
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_text_selection_tests.rs"]
mod tests;
