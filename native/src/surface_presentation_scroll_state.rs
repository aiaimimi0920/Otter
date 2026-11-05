//! 受信宿主的只读滚动快照与原子重投影；不是网络提供的偏移，也不授予事件权限。
use super::*;
impl Presentation {
    pub(crate) fn reveal_focus_in(
        &mut self,
        id: &str,
        viewport: crate::surface_clip::Rect,
    ) -> Result<bool> {
        if !self.can_focus(id) || viewport.empty() {
            return Ok(false);
        }
        // 内层scrollport可能先在crop外；外层显露后再核内层，次数受场景深度门约束。
        let before = self.scroll_state();
        for _ in 0..crate::surface_scene::MAX_DEPTH {
            let Some(g) = self.geometry.nodes().iter().find(|g| g.id() == id) else {
                break;
            };
            let mut scroll = self.scroll.clone();
            if !scroll.reveal_region_in(&self.geometry, id, g.origin(), g.size(), Some(viewport)) {
                break;
            }
            self.restore_scroll(&scroll)?;
        }
        let visible = self
            .geometry
            .nodes()
            .iter()
            .find(|g| g.id() == id)
            .is_some_and(|g| {
                !crate::surface_clip::Rect::from_box(g.origin(), g.size())
                    .intersect(g.clip())
                    .intersect(viewport)
                    .empty()
            });
        if !visible {
            self.restore_scroll(&before)?;
        }
        Ok(visible)
    }
    pub(crate) fn scroll_state(&self) -> crate::surface_scroll::Scroll {
        self.scroll.clone()
    }
    pub(crate) fn planned_scrollbar(
        &self,
        id: &str,
        axis: usize,
        value: f32,
    ) -> Option<crate::surface_scroll::Scroll> {
        let mut scroll = self.scroll.clone();
        scroll.set_bar(id, axis, value).then_some(scroll)
    }
    pub(crate) fn layout_identity(&self) -> [u8; 32] {
        self.cache.identity
    }
    pub(crate) fn restore_scroll(&mut self, saved: &crate::surface_scroll::Scroll) -> Result<()> {
        let mut scroll = saved.clone();
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        self.install_frame(frame, scroll);
        Ok(())
    }
    pub(crate) fn planned_wheel(
        &self,
        point: [f32; 2],
        axis: usize,
        delta: f32,
        page: bool,
    ) -> Option<crate::surface_scroll::Scroll> {
        let hit = self
            .scrollbar_at(point)
            .map(|bar| bar.id)
            .or_else(|| self.hit_test(point).map(str::to_owned))?;
        let mut scroll = self.scroll.clone();
        scroll
            .wheel(&self.geometry, &hit, axis, delta, page)
            .then_some(scroll)
    }
}
#[cfg(test)]
#[path = "surface_presentation_scroll_state_tests.rs"]
mod tests;
