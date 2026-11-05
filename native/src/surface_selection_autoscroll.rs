//! 拖选边缘滚动复用祖先 wheel 与缓存投影；只移动当前选区 owner，不重测文字。
use super::*;
impl Presentation {
    fn text_drag_scroll_candidate(&self, point: [f32; 2]) -> Option<crate::surface_scroll::Scroll> {
        if point.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let selection = self.selection.as_ref().filter(|s| s.dragging)?;
        let item = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.focus_node && item.selectable())?;
        let clip = item.clip();
        if clip.empty() {
            return None;
        }
        let mut scroll = self.scroll.clone();
        let mut changed = false;
        for (axis, (low, high)) in [(clip.left, clip.right), (clip.top, clip.bottom)]
            .into_iter()
            .enumerate()
        {
            let edge = 12f32.min((high - low) / 4.);
            let delta = if point[axis] < low + edge {
                -16.
            } else if point[axis] > high - edge {
                16.
            } else {
                0.
            };
            changed |= scroll.wheel(&self.geometry, &selection.focus_node, axis, delta, false);
        }
        changed.then_some(scroll)
    }
    pub(crate) fn text_drag_scroll_pending(&self, point: [f32; 2]) -> bool {
        self.text_drag_scroll_candidate(point).is_some()
    }
    pub(crate) fn step_text_drag_scroll(&mut self, point: [f32; 2]) -> Result<bool> {
        let Some(mut scroll) = self.text_drag_scroll_candidate(point) else {
            return Ok(false);
        };
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        let node = &self.selection.as_ref().unwrap().focus_node;
        if !frame
            .text
            .items()
            .iter()
            .any(|item| item.node_id() == node && item.selectable() && !item.clip().empty())
        {
            return Ok(false);
        }
        self.install_frame(frame, scroll);
        self.update_text_selection(point)?;
        Ok(true)
    }
}
