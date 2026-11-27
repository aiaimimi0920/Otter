//! Ctrl+E/F4 control only existing edit-tool panels; do not take Tab, open parameters/Art, or inject global input.
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
    !state.click.active()
        && !state.session_end.paused()
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
        // fit/ShowWindow may synchronously reenter; do not reuse destroyed/replaced hosts or bypass modal gates afterward.
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
    Some(LRESULT(0)) // Consume auto-repeat too; do not pass it through as edit commands or repeated visibility toggles.
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
            // The host uniformly deletes idle pins on Esc; property EDIT/modal menus already consumed cancellation during pretranslation.
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
