//! Surface system-session seam: await accepted-input settlement; after approval reject new business work while preserving original destruction cleanup.
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
impl SurfaceWindow {
    pub(in crate::desktop) fn session_end_idle(&self) -> bool {
        let capture = unsafe { GetCapture() };
        self.is_alive()
            && !self.state.pointer.borrow().automation_busy()
            && self.state.keyboard.borrow().pressed().is_none()
            && self.state.scroll_drag.borrow().is_none()
            && self.state.scroll_repeat.borrow().is_none()
            && self.state.pan.borrow().is_none()
            && capture != self.hwnd
            && !unsafe { IsChild(self.hwnd, capture) }.as_bool()
            && self
                .state
                .host
                .borrow()
                .as_ref()
                .is_some_and(Host::session_end_idle)
    }
}
impl State {
    pub(super) unsafe fn session_message(
        &self,
        hwnd: HWND,
        message: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> Option<LRESULT> {
        crate::desktop::session_end::dispatch_active(
            hwnd,
            &self.session_end_reason,
            message,
            wp,
            lp,
        )
    }
}
