//! 窗口像素到当前文本布局的桥接；复用 viewport 同步，不保存第二份选区。
use super::*;
impl Host {
    pub(in crate::desktop) fn begin_word_selection(&mut self, pixels: [i32; 2]) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        self.presentation
            .begin_word_selection(pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32))
    }
    pub(in crate::desktop) fn text_drag_scroll(
        &mut self,
        pixels: [i32; 2],
        step: bool,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let point = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        if !step {
            return Ok(self.presentation.text_drag_scroll_pending(point));
        }
        let changed = self.presentation.step_text_drag_scroll(point)?;
        if changed {
            self.sync_edits()?;
        }
        Ok(changed)
    }
    pub(in crate::desktop) fn line_edge_text_selection(
        &mut self,
        node: &str,
        end: bool,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let handled = self.presentation.line_edge_text_selection(node, end)?;
        if handled && self.presentation.reveal_text_selection()? {
            self.sync_edits()?;
        }
        Ok(handled)
    }
    pub(in crate::desktop) fn vertical_text_selection(
        &mut self,
        node: &str,
        down: bool,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let handled = self.presentation.vertical_text_selection(node, down)?;
        if handled && self.presentation.reveal_text_selection()? {
            self.sync_edits()?;
        }
        Ok(handled)
    }
    pub(in crate::desktop) fn step_text_selection(
        &mut self,
        node: &str,
        forward: bool,
        extend: bool,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let handled = self
            .presentation
            .step_text_selection(node, forward, extend)?;
        if handled && self.presentation.reveal_text_selection()? {
            self.sync_edits()?;
        }
        Ok(handled)
    }
    pub(in crate::desktop) fn selectable_text_at(&mut self, pixels: [i32; 2]) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        self.presentation
            .selectable_text_at(pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32))
    }
    pub(in crate::desktop) fn selected_text(&mut self) -> Result<Option<String>> {
        if !self.sync_viewport()? {
            return Ok(None);
        }
        self.presentation.selected_text()
    }
    pub(in crate::desktop) fn begin_text_selection(&mut self, pixels: [i32; 2]) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        self.presentation
            .begin_text_selection(pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32))
    }
    pub(in crate::desktop) fn update_text_selection(
        &mut self,
        pixels: [i32; 2],
        finish: bool,
    ) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let point = pixels.map(|v| v as f32 * 96. / self.viewport.1 as f32);
        if finish {
            self.presentation.finish_text_selection(point)
        } else {
            self.presentation.update_text_selection(point)
        }
    }
    pub(in crate::desktop) fn cancel_text_selection(&mut self, completed: bool) {
        if completed {
            self.presentation.clear_text_selection();
        } else {
            self.presentation.cancel_text_drag();
        }
    }
}
