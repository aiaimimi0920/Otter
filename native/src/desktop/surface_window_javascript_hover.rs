//! 无按键悬停只采样当前 HWND 命中；不夺焦点/捕获，和原鼠标视觉共享离开跟踪。
use super::*;
use crate::surface_javascript_wire::HoverInput;
pub(in crate::desktop::surface_window) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) {
    if !state.javascript_keyboard.get()
        || !matches!(message, WM_MOUSEMOVE | WM_MOUSELEAVE | WM_LBUTTONUP)
    {
        return;
    }
    if state.javascript_pointer.borrow().active.is_some() {
        return;
    }
    // WM_MOUSEMOVE 的五个按键位；Ctrl/Shift 不属于按键，仍由原 modifier owner 采样。
    if message != WM_MOUSELEAVE && wp.0 & 0x73 != 0 {
        return;
    }
    if !hover_eligible(hwnd, state) {
        cancel(state);
        return;
    }
    // 原生控件/选择/scrollbar 的捕获不由悬停路径抢占；Canvas up 已在调用前释放。
    if GetCapture() != HWND::default() {
        return;
    }
    let mut pixels = point(lp);
    if message == WM_MOUSELEAVE {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_ok() && ScreenToClient(hwnd, &mut cursor).as_bool() {
            pixels = [cursor.x, cursor.y];
        }
    }
    let (client, modifiers) = data(hwnd, pixels);
    let leaving = message == WM_MOUSELEAVE;
    let result = state.run(|host| {
        // Canvas up 只续结已存在的 hover，不为纯拖动凭空增加输入。
        if message == WM_LBUTTONUP && !host.has_javascript_hovers() {
            return Ok(());
        }
        let target = if leaving {
            None
        } else {
            host.hover_target(pixels)?
        };
        host.javascript_hover(
            HoverInput {
                target,
                client,
                modifiers,
            },
            state.viewport_epoch.get(),
        )
    });
    match result {
        Ok(false) if leaving => state.pointer_cancel_pending.set(true),
        Err(error) => *state.error.borrow_mut() = Some(error),
        _ => {}
    }
    if message == WM_MOUSEMOVE {
        pointer::track_leave(hwnd, state);
    }
}
