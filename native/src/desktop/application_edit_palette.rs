//! After the color panel closes, reset the other pins' creation-default slots within the same outer message settlement.
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
