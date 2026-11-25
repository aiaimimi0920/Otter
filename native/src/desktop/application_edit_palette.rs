//! 颜色面板关闭后，在同一外层消息结算中重置其余钉图的创建默认槽。
use super::*;
impl Application {
    pub(super) unsafe fn settle_edit_palette(&self) {
        let pending = self.edit_palette.borrow_mut().take_reset();
        let Some(pending) = pending else {
            return;
        };
        for doc in &self.documents {
            if doc.is_alive() && doc.state.unit_id != pending.origin {
                super::super::edit_palette::reset(
                    &mut doc.state.canvas.borrow_mut(),
                    &pending.removed,
                );
                super::super::tool_panel::refresh(doc.window.0);
            }
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_edit_palette_tests.rs"]
mod tests;
