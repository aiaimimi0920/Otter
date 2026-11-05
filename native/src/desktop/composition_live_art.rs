//! Live Add Art 入口只交单槽意图；先停止源输入，选择与建窗由 Application 持有。
use super::*;
use crate::desktop::node_menu::art_picker;
const ADD: usize = 0x1f80;
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    AppendMenuW(
        GetSystemMenu(hwnd, false),
        MF_STRING,
        ADD,
        w!("添加 Art…\tShift+1"),
    )
}
pub(super) unsafe fn handle(hwnd: HWND, state: &State, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    let key = art_picker::key(msg, wp);
    if !(key || msg == WM_SYSCOMMAND && wp.0 & 0xfff0 == ADD) {
        return false;
    }
    if key && lp.0 & (1 << 30) != 0 {
        return true;
    }
    // Shift 可能已送到源；沿既有停止通道释放，不让模态键盘继续控制用户源窗口。
    input::cancel(hwnd, state);
    if !state.closed.get()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && !state.menu_open.get()
        && state.drag.get().is_none()
        && !state.drag_starting.get()
        && !state.freeze_requested.get()
        && !state.recycle_requested.get()
    {
        state.art_requested.set(true);
    }
    true
}
impl LiveWindow {
    pub(in crate::desktop) fn art_pending(&self) -> bool {
        self.state.art_requested.get()
    }
    pub(in crate::desktop) fn take_art_request(&self) -> bool {
        self.state.art_requested.replace(false)
    }
    pub(in crate::desktop) fn art_dialog_owner(&self) -> Option<HWND> {
        (self.persistence_idle()
            && !self.group_input_blocked()
            && !unsafe { IsIconic(self.hwnd).as_bool() })
        .then_some(self.hwnd)
    }
}
#[cfg(test)]
#[path = "composition_live_art_tests.rs"]
mod tests;
