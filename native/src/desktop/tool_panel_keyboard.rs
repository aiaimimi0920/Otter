//! Ctrl+E 与 F4 只控制既有编辑工具面板；不抢 Tab、不打开参数/Art，也不注入全局输入。
use super::*;

fn toggles(message: u32, key: usize, control: bool, shift: bool, alt: bool) -> bool {
    message == WM_KEYDOWN && (key == VK_F4.0 as usize || (key == 0x45 && control && !shift && !alt))
}
unsafe fn toggle_key(message: u32, key: usize) -> bool {
    toggles(
        message,
        key,
        GetKeyState(VK_CONTROL.0 as i32) < 0,
        GetKeyState(VK_SHIFT.0 as i32) < 0,
        GetKeyState(VK_MENU.0 as i32) < 0,
    )
}
fn first_press(lp: LPARAM) -> bool {
    lp.0 & (1 << 30) == 0
}
unsafe fn current(owner: HWND, panel: HWND, state: &State) -> bool {
    state.lifetime.owns(owner)
        && IsWindow(Some(owner)).as_bool()
        && IsWindow(Some(panel)).as_bool()
        && state.tool_panel.get() == panel
        && GetWindowLongPtrW(owner, GWLP_USERDATA) == state as *const State as isize
        && GetWindowLongPtrW(panel, GWLP_USERDATA) == owner.0 as isize
}
pub(super) unsafe fn owner(state: &State) -> Option<HWND> {
    let panel = state.tool_panel.get();
    let owner = HWND(GetWindowLongPtrW(panel, GWLP_USERDATA) as *mut _);
    current(owner, panel, state).then_some(owner)
}
fn available(state: &State) -> bool {
    !state.session_end.paused()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && super::super::capture_input::exclusive_available(state)
}

pub(super) unsafe fn dispatch(
    owner: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !toggle_key(message, wp.0) {
        return None;
    }
    let panel = state.tool_panel.get();
    if first_press(lp) && current(owner, panel, state) && available(state) {
        let showing = !IsWindowVisible(panel).as_bool();
        if showing {
            if let Err(error) = position::fit(panel, None) {
                eprintln!("native-tool-panel-position: {error}");
            }
        }
        // fit/ShowWindow 可同步重入；不能沿用已销毁/替换宿主或在重入后绕过模态门。
        if current(owner, panel, state) && available(state) {
            let _ = ShowWindow(panel, if showing { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }
    Some(LRESULT(0)) // 自动重复仍消费，不能透传成编辑命令或多次显隐。
}

pub(super) unsafe fn translate(state: &State, message: &MSG) -> bool {
    if message.message != WM_KEYDOWN
        || !(message.wParam.0 == VK_ESCAPE.0 as usize
            || toggle_key(message.message, message.wParam.0))
    {
        return false;
    }
    let panel = state.tool_panel.get();
    let owner = HWND(GetWindowLongPtrW(panel, GWLP_USERDATA) as *mut _);
    if !current(owner, panel, state)
        || !(message.hwnd == panel || IsChild(panel, message.hwnd).as_bool())
    {
        return false;
    }
    if first_press(message.lParam) && available(state) && IsWindowVisible(panel).as_bool() {
        // 复用面板原 WM_CLOSE：只隐藏并将线程焦点还给同一 owner，不请求全局前台。
        let _ = SendMessageW(panel, WM_CLOSE, None, None);
    }
    true
}

#[cfg(test)]
#[path = "tool_panel_keyboard_tests.rs"]
mod tests;
