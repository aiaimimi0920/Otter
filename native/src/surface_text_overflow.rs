//! Apply ellipsis only to final text frames; original layout owners retain intrinsic measurement, complete UTF-16, and box geometry.
use super::*;
use crate::surface_scene::{Kind, Node};

impl Engine {
    pub(crate) fn layout_final(
        &self,
        node: &Node,
        text: &str,
        style: Style<'_>,
        width: f32,
        height: f32,
    ) -> Result<Layout> {
        let ellipsis = match node.style().get("textOverflow") {
            None => false,
            Some(value) => match value.as_str() {
                Some("clip") => false,
                Some("ellipsis") => true,
                _ => return Err(invalid()),
            },
        };
        if ellipsis
            && (node.kind() != Kind::Text
                || style.whitespace != Whitespace::Nowrap
                || !matches!(style.align, Align::Start)
                || ["overflowX", "overflowY"]
                    .iter()
                    .any(|key| node.layout().get(*key).and_then(|v| v.as_str()) != Some("hidden")))
        {
            // Multiline/other-alignment CSS inline-end differs from DWrite end-of-line trimming; never approximate silently.
            return Err(invalid());
        }
        let layout = self.layout(text, style, width, height)?;
        if ellipsis {
            // nowrap may still contain mandatory Unicode separators; do not admit actual multiline content as a single line.
            if layout.line_count() != 1 {
                return Err(invalid());
            }
            // This layout is not shared yet; never mutate COM objects belonging to previous frames or intrinsic measurement in place.
            // text-overflow does not truncate the final vertical line: native height holds at least one whole line, with actual pixels clipped by TextItem.
            // Retain pre-trimming metrics; ellipsis affects only painting/hits, not intrinsic width or overflow geometry.
            unsafe {
                layout
                    .native
                    .SetMaxHeight(height.max(layout.metrics.height))?;
                let sign = self.factory.CreateEllipsisTrimmingSign(&layout.native)?;
                layout.native.SetTrimming(
                    &DWRITE_TRIMMING {
                        granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                        ..Default::default()
                    },
                    &sign,
                )?;
            }
        }
        Ok(layout)
    }
}
impl Layout {
    /// DWrite may return trimming signs as zero-height inline hits; use text hits from the same logical range
    /// to recover actual line height, neither treating isInside=false as unselectable ellipsis nor expanding to the entire box.
    pub(super) fn normalize_trimmed_hit(&self, hit: &mut DWRITE_HIT_TEST_METRICS) -> Result<()> {
        if !hit.isTrimmed.as_bool() || (hit.isText.as_bool() && hit.height > 0.) {
            return Ok(());
        }
        if hit.textPosition >= self.text_len() {
            return Err(invalid());
        }
        let (mut x, mut y) = (0., 0.);
        let mut text_hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native.HitTestTextPosition(
                hit.textPosition,
                false,
                &mut x,
                &mut y,
                &mut text_hit,
            )?;
        }
        if !text_hit.isTrimmed.as_bool() || text_hit.height <= 0. || text_hit.length == 0 {
            return Err(invalid());
        }
        *hit = text_hit;
        Ok(())
    }
    /// Ellipsis represents the entire hidden suffix; word selection must not copy only one invisible word within it.
    pub(super) fn omitted_range(&self, position: u32) -> Result<Option<std::ops::Range<u32>>> {
        if position > self.text_len() {
            return Err(invalid());
        }
        if self.text.is_empty() {
            return Ok(None);
        }
        let (mut x, mut y) = (0., 0.);
        let mut hit = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            self.native.HitTestTextPosition(
                position.min(self.text_len() - 1),
                false,
                &mut x,
                &mut y,
                &mut hit,
            )?;
        }
        if !hit.isTrimmed.as_bool() {
            return Ok(None);
        }
        let end = hit
            .textPosition
            .checked_add(hit.length)
            .filter(|end| *end <= self.text_len())
            .ok_or_else(invalid)?;
        if hit.length == 0 {
            return Err(invalid());
        }
        Ok(Some(hit.textPosition..end))
    }
}
