//! Keyboard selection-endpoint reveal prepares only scroll projection, committing after candidate visibility checks without remeasuring text.
use super::*;
use crate::surface_clip::Rect;
impl Selection {
    fn focus_region(&self, text: &TextFrame) -> Result<Option<Rect>> {
        let Some(item) = text
            .items()
            .iter()
            .find(|item| item.node_id() == self.focus_node && item.selectable())
        else {
            return Ok(None);
        };
        let len = item.layout().text_len();
        if len == 0 {
            return Ok(None);
        }
        // Forward extension reveals the last selected cluster; reverse or collapsed selection reveals the cluster at the current insertion edge.
        let index = if self.forward(text) && self.focus > 0 {
            self.focus - 1
        } else {
            self.focus.min(len - 1)
        };
        let rect = if let Some(point) = self.vertical_point {
            // A soft-wrap index may belong to two visual lines; reveal must use the actual line saved by vertical navigation.
            item.layout().point_cluster_rect(point)?
        } else {
            item.layout().cluster_rect(index)?
        };
        let origin = item.origin();
        Ok(Some(Rect {
            left: rect.left + origin[0],
            right: rect.right + origin[0],
            top: rect.top + origin[1],
            bottom: rect.bottom + origin[1],
        }))
    }
}
impl Presentation {
    pub(crate) fn reveal_text_selection(&mut self) -> Result<bool> {
        let Some(selection) = self.selection.as_ref().filter(|s| !s.dragging) else {
            return Ok(false);
        };
        let Some(rect) = selection.focus_region(&self.text)? else {
            return Ok(false);
        };
        let mut scroll = self.scroll.clone();
        if !scroll.reveal_region(
            &self.geometry,
            &selection.focus_node,
            [rect.left, rect.top],
            [rect.right - rect.left, rect.bottom - rect.top],
        ) {
            return Ok(false);
        }
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        let region = selection.focus_region(&frame.text)?;
        let clip = frame
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.focus_node)
            .map(TextItem::clip);
        if region
            .zip(clip)
            .is_none_or(|(region, clip)| region.intersect(clip).empty())
        {
            // If zero area or static clipping makes a target unreachable, failed candidates must not move outer views.
            return Ok(false);
        }
        self.install_frame(frame, scroll);
        Ok(true)
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_selection_reveal_tests.rs"]
mod tests;
