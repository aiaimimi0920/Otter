//! Canvas 命中/捕获输入借用原呈现和编辑 FIFO，不持独立节点索引。
use super::*;
use crate::surface_javascript_wire::PointerInput;
impl Host {
    pub(crate) fn has_javascript_hovers(&self) -> bool {
        self.edits.has_javascript_hovers()
    }
    pub(crate) fn javascript_hover(
        &mut self,
        input: crate::surface_javascript_wire::HoverInput,
        epoch: u64,
    ) -> Result<()> {
        self.edits.javascript_hover(input, epoch)
    }
    pub(crate) fn hover_target(&mut self, pixels: [i32; 2]) -> Result<Option<String>> {
        let target = self.hit_test(pixels)?.map(str::to_owned);
        Ok(target.filter(|id| {
            crate::desktop::surface_edit_mount::hover::dom_node(&self.presentation, id)
        }))
    }
    #[cfg(test)]
    pub(crate) fn javascript_pointer_front_for_test(
        &self,
    ) -> Option<crate::surface_javascript_wire::PointerPhase> {
        self.edits.javascript_pointer_front_for_test()
    }
    pub(crate) fn cancel_javascript_pointers(&mut self, epoch: u64) -> Result<()> {
        self.edits.cancel_javascript_pointers(epoch)
    }
    pub(crate) fn canvas_pointer_target(&mut self, pixels: [i32; 2]) -> Result<Option<String>> {
        let target = self.hit_test(pixels)?.map(str::to_owned);
        Ok(target.filter(|id| self.canvas_pointer_current(id)))
    }
    pub(crate) fn canvas_pointer_current(&self, id: &str) -> bool {
        crate::desktop::surface_edit_mount::pointer::canvas_node(&self.presentation, id)
    }
    pub(crate) fn javascript_pointer(
        &mut self,
        input: PointerInput,
        gesture: u64,
        epoch: u64,
    ) -> Result<()> {
        self.edits.javascript_pointer(input, gesture, epoch)
    }
}
