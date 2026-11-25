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
        && panel_owner(panel) == owner
}
unsafe fn panel_owner(panel: HWND) -> HWND {
    neuro_mini_ui::owner(panel)
        .unwrap_or_else(|| HWND(GetWindowLongPtrW(panel, GWLP_USERDATA) as *mut _))
}
pub(super) unsafe fn owner(state: &State) -> Option<HWND> {
    let panel = state.tool_panel.get();
    let owner = panel_owner(panel);
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
        if super::super::edit_mode::shortcut(owner, state, message, wp.0, !first_press(lp)) {
            return Some(LRESULT(0));
        }
        return None;
    }
    let panel = state.tool_panel.get();
    if first_press(lp) && current(owner, panel, state) && available(state) {
        let showing = !IsWindowVisible(panel).as_bool();
        if showing {
            if let Err(error) = prepare(panel, state) {
                eprintln!("native-tool-panel-prepare: {error}");
                return Some(LRESULT(0));
            }
            if let Err(error) = if neuro_mini_ui::is_toolbar(panel) {
                strip::place(owner, state, None)
            } else {
                position::fit(panel, None)
            } {
                eprintln!("native-tool-panel-position: {error}");
            }
        }
        // fit/ShowWindow 可同步重入；不能沿用已销毁/替换宿主或在重入后绕过模态门。
        if current(owner, panel, state) && available(state) {
            if neuro_mini_ui::is_toolbar(panel) {
                if let Err(error) = super::super::edit_mode::toolbar_mode(owner, state, showing) {
                    eprintln!("native-tool-panel-mode: {error}");
                    return Some(LRESULT(0));
                }
            }
            let _ = ShowWindow(panel, if showing { SW_SHOWNOACTIVATE } else { SW_HIDE });
            if showing && neuro_mini_ui::is_toolbar(panel) {
                if let Err(error) = super::super::extension_catalog::request(owner, state) {
                    eprintln!("native-extension-catalog-request: {error}");
                }
            }
        }
    }
    Some(LRESULT(0)) // 自动重复仍消费，不能透传成编辑命令或多次显隐。
}

pub(super) unsafe fn translate(state: &State, message: &MSG) -> bool {
    let panel = state.tool_panel.get();
    let owner = panel_owner(panel);
    if !current(owner, panel, state)
        || !(message.hwnd == panel || IsChild(panel, message.hwnd).as_bool())
    {
        return false;
    }
    if super::super::clipboard_input::translate(owner, message) {
        return true;
    }
    if message.message != WM_KEYDOWN
        || !(message.wParam.0 == VK_ESCAPE.0 as usize
            || toggle_key(message.message, message.wParam.0))
    {
        return false;
    }
    if first_press(message.lParam) && available(state) && IsWindowVisible(panel).as_bool() {
        if message.wParam.0 == VK_ESCAPE.0 as usize && neuro_mini_ui::is_toolbar(panel) {
            // 空闲钉图 Esc 由宿主统一删除；属性 EDIT 和模态菜单已在前置翻译中消费取消。
            SendMessageW(
                owner,
                WM_KEYDOWN,
                Some(message.wParam),
                Some(message.lParam),
            );
        } else {
            let _ = SendMessageW(panel, WM_CLOSE, None, None);
        }
    }
    true
}

pub(super) unsafe fn translate_property(state: &State, message: &MSG) -> bool {
    if !neuro_mini_ui::property_contains(state.tool_panel.get(), message.hwnd) {
        return false;
    }
    if let Some(owner) = owner(state) {
        if super::super::clipboard_input::translate(owner, message) {
            return true;
        }
    }
    if toggle_key(message.message, message.wParam.0) {
        if let Some(owner) = owner(state) {
            dispatch(
                owner,
                state,
                message.message,
                message.wParam,
                message.lParam,
            );
            return true;
        }
    }
    neuro_mini_ui::translate_properties(state.tool_panel.get(), message)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "tool_panel_keyboard_tests.rs"]
mod tests;
