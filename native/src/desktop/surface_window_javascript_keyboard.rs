//! Synchronous messages capture only Enter; asynchronous defaults settle in the original field owner, without child waits or WM_KEYDOWN replay.
use super::*;
use crate::desktop::surface_edit_messages::ENTER_KEY;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled};
impl SurfaceWindow {
    pub(crate) fn enable_javascript_keyboard(&self) {
        self.state.javascript_keyboard.set(true);
    }
    pub(crate) fn complete_javascript_edit(&self, id: &str, allowed: Option<bool>) -> Result<()> {
        let focus = self
            .javascript_focus_allowed()
            .then(|| (unsafe { GetFocus() }, self.javascript_focus_epoch()));
        if !self.is_alive()
            || !self
                .state
                .run(|host| host.complete_javascript_edit(id, allowed, focus))?
        {
            return Err(Error::new(
                E_FAIL,
                "JavaScript input completion unavailable",
            ));
        }
        Ok(())
    }
}
pub(super) unsafe fn message(
    parent: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if message != ENTER_KEY || !state.javascript_keyboard.get() {
        return None;
    }
    let child = HWND(lp.0 as *mut _);
    // JS windows consume Enter even after revocation, preventing fallback to synchronous COMMIT that bypasses listeners.
    if wp.0 <= 127
        && !state.closed.get()
        && state.local_edit.get()
        && GetFocus() == child
        && GetParent(child).ok() == Some(parent)
        && IsWindowEnabled(child).as_bool()
        && IsWindowVisible(child).as_bool()
        && GetForegroundWindow() == parent
    {
        match state.run(|host| host.javascript_enter(child, wp.0 as u32, state.focus_epoch.get())) {
            Ok(true) => {}
            Ok(false) => {
                *state.error.borrow_mut() =
                    Some(Error::new(E_FAIL, "Surface keyboard owner unavailable"))
            }
            Err(error) => *state.error.borrow_mut() = Some(error),
        }
    }
    Some(LRESULT(1))
}
