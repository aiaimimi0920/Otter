//! 同步消息只捕获 Enter；异步默认动作在原字段 owner 结算，不等待 child 或重放 WM_KEYDOWN。
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
    // JS 窗口即使已撤权也消费 Enter，不能落回同步 COMMIT 绕开 listener。
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
