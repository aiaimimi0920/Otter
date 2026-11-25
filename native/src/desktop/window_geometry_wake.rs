//! Sent geometry messages do not guarantee GetMessage returns; coalesce one queued notification per HWND, leaving disk/model settlement to the application.
use std::cell::Cell;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub(super) const MESSAGE: u32 = WM_APP + 117;
pub(super) struct Wake {
    active: Cell<bool>,
    pending: Cell<bool>,
}
impl Default for Wake {
    fn default() -> Self {
        Self {
            active: Cell::new(true),
            pending: Cell::new(false),
        }
    }
}
impl Wake {
    pub(super) fn acknowledge(&self) {
        self.pending.set(false);
    }
    fn request(&self, flags: SET_WINDOW_POS_FLAGS, send: impl FnOnce() -> bool) {
        if self.active.get() && !self.pending.get() && !flags.contains(SWP_NOMOVE | SWP_NOSIZE) {
            // PostMessage does not synchronously reenter window procedures; failure does not latch pending, and existing queued messages can drive settlement.
            self.pending.set(send());
        }
    }
    fn close(&self) {
        self.active.set(false);
        self.pending.set(false);
    }
}
pub(super) unsafe fn dispatch(hwnd: HWND, wake: &Wake, msg: u32, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_WINDOWPOSCHANGED if lp.0 != 0 => {
            wake.request((*(lp.0 as *const WINDOWPOS)).flags, || {
                PostMessageW(Some(hwnd), MESSAGE, WPARAM(0), LPARAM(0)).is_ok()
            });
            // Continue DefWindowProc's default WM_MOVE/WM_SIZE dispatch; do not swallow resize/DPI/layout.
            None
        }
        WM_DESTROY | WM_NCDESTROY => {
            wake.close(); // Even while State survives, never post to destroyed, potentially reused HWNDs.
            None
        }
        _ => None,
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "window_geometry_wake_tests.rs"]
mod tests;
