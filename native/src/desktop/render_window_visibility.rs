//! Rebuild visibility from CPU layered content to GPU; temporary hiding preserves activation, position, and Z-order, and all writes require a stable host.
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) struct Reset<'a> {
    hwnd: HWND,
    restore: bool,
    owns: &'a dyn Fn() -> bool,
    allow_restore: &'a dyn Fn() -> bool,
    _thread: PhantomData<Rc<()>>,
}

impl<'a> Reset<'a> {
    /// # Safety
    /// The HWND must belong to this thread; the old GPU is retired and the new GPU unattached. Hold no Canvas borrow across the call.
    /// removed_layered must come from this committed style transaction; do not repeatedly hide for ordinary GPU repaint.
    pub(super) unsafe fn begin(
        hwnd: HWND,
        removed_layered: bool,
        owns: &'a dyn Fn() -> bool,
        allow_restore: &'a dyn Fn() -> bool,
    ) -> Result<Self> {
        if !owns() {
            return Err(Error::new(E_ABORT, "Render visibility owner was revoked"));
        }
        let reset = Self {
            hwnd,
            restore: removed_layered && IsWindowVisible(hwnd).as_bool(),
            owns,
            allow_restore,
            _thread: PhantomData,
        };
        if reset.restore {
            // NOACTIVATE was verified to preserve this thread's active/focus windows and other applications' foreground; SW_HIDE provides no such contract.
            position(hwnd, SWP_HIDEWINDOW)?;
            if !owns() {
                return Err(Error::new(E_ABORT, "Render visibility owner was revoked"));
            }
        }
        Ok(reset)
    }

    pub(super) unsafe fn restore(&mut self) -> Result<()> {
        // One attempt; on failure do not flood retries in Drop or overwrite group/tray hiding intents produced by reentrancy.
        if !std::mem::take(&mut self.restore) || !(self.owns)() || !(self.allow_restore)() {
            return Ok(());
        }
        position(self.hwnd, SWP_SHOWWINDOW)?;
        // SHOW itself can reenter; the outer SHOW must not overwrite new hiding intents created inside it.
        // Correct only once, without battling reentrant business loops over visibility; never write after destruction.
        if (self.owns)() && !(self.allow_restore)() {
            position(self.hwnd, SWP_HIDEWINDOW)?;
        }
        Ok(())
    }
}

unsafe fn position(hwnd: HWND, visibility: SET_WINDOW_POS_FLAGS) -> Result<()> {
    SetWindowPos(
        hwnd,
        None,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE | visibility,
    )
}

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        if let Err(error) = unsafe { self.restore() } {
            eprintln!("native-render-visibility-restore: {error}");
        }
    }
}

#[cfg(test)]
#[path = "render_window_visibility_tests.rs"]
mod tests;
