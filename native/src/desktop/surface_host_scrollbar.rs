//! HWND 像素与滚动条 DIP 接缝；沿用呈现事务和 EDIT 重定位。
use super::*;
use crate::surface_scroll::{Bar, Drag};
impl Host {
    pub(in crate::desktop) fn pan_anchor(&mut self, pixels: [i32; 2]) -> Result<Option<String>> {
        if !self.sync_viewport()? {
            return Ok(None);
        }
        let point = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        Ok(self
            .presentation
            .scrollbar_at(point)
            .map(|bar| bar.id)
            .or_else(|| self.presentation.hit_test(point).map(str::to_owned)))
    }
    pub(in crate::desktop) fn pan(&mut self, anchor: &str, pixels: [i32; 2]) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let delta = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        let changed = self.presentation.pan(anchor, delta)?;
        if changed {
            self.sync_edits()?;
        }
        Ok(changed)
    }
    pub(in crate::desktop) fn scroll_key(
        &mut self,
        anchor: Option<&str>,
        command: crate::surface_scroll::Command,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let changed = self.presentation.scroll_key(anchor, command)?;
        if changed {
            self.sync_edits()?;
        }
        Ok(changed)
    }
    pub(in crate::desktop) fn page_scrollbar(
        &mut self,
        id: &str,
        axis: usize,
        direction: i8,
        pixels: [i32; 2],
    ) -> Result<bool> {
        let before = self.viewport;
        let hit = self.scrollbar(pixels)?;
        // 命中查询可能先更新呈现几何，即使本次不翻页也要保留重绘。
        let reflowed = before.0.width != self.viewport.0.width
            || before.0.height != self.viewport.0.height
            || before.1 != self.viewport.1;
        if let Some((bar, point)) = hit {
            // thumb 到达指针后不反向翻页，也不转交重叠的另一条。
            if bar.id == id
                && bar.axis == axis
                && direction != 0
                && bar.page_direction(point) == direction
            {
                return Ok(self.set_scrollbar(id, axis, bar.page_value(point))? || reflowed);
            }
        }
        Ok(reflowed)
    }
    pub(in crate::desktop) fn scrollbar(
        &mut self,
        pixels: [i32; 2],
    ) -> Result<Option<(Bar, [f32; 2])>> {
        if !self.sync_viewport()? {
            return Ok(None);
        }
        let point = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        Ok(self
            .presentation
            .scrollbar_at(point)
            .map(|bar| (bar, point)))
    }
    pub(in crate::desktop) fn drag_scrollbar(
        &mut self,
        drag: &Drag,
        pixels: [i32; 2],
    ) -> Result<()> {
        if !self.sync_viewport()? {
            return Ok(());
        }
        let point = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        self.set_scrollbar(&drag.id, drag.axis, drag.value(point))
            .map(|_| ())
    }
    fn set_scrollbar(&mut self, id: &str, axis: usize, value: f32) -> Result<bool> {
        let changed = self.presentation.set_scrollbar(id, axis, value)?;
        if changed {
            self.sync_edits()?;
        }
        Ok(changed)
    }
}
