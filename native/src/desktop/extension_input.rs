//! 原钉图消息中的覆盖文字选择/复制与声明 click；不另建输入窗口、hook 或渲染层。
use super::{
    extension_visuals::{Outcome, Views},
    State,
};
use crate::{editor::Tool, surface_click_event::Modifiers};
use std::rc::Rc;
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::ScreenToClient,
        UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

fn allowed(state: &State) -> bool {
    let c = state.canvas.borrow();
    c.pinned
        && c.minified.is_none()
        && !c.crop_mode
        && !c.blocks_wheel()
        && !state.session_end.paused()
        && !state.group_hidden.get()
        && !state.modals.blocked()
        && if unsafe { IsWindowVisible(state.tool_panel.get()).as_bool() } {
            c.tool == Tool::Select && c.selection_enabled
        } else {
            c.tool == Tool::Pan
        }
}
pub(super) fn views(state: &State) -> Option<Rc<Views>> {
    if !allowed(state) || !super::extension_overlay::current(state) {
        return None;
    }
    let c = state.canvas.borrow();
    c.extension_views
        .as_ref()?
        .views(&c, state.resolved_display.borrow().as_ref())
}
fn same(state: &State, old: &Rc<Views>) -> bool {
    views(state).is_some_and(|new| Rc::ptr_eq(old, &new))
}
unsafe fn logical(hwnd: HWND, lp: LPARAM) -> [f32; 2] {
    let factor = 96. / GetDpiForWindow(hwnd).max(96) as f32;
    [
        lp.0 as i16 as f32 * factor,
        (lp.0 >> 16) as i16 as f32 * factor,
    ]
}
pub(super) unsafe fn screen_hit(hwnd: HWND, state: &State, lp: LPARAM) -> bool {
    let mut point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    if !ScreenToClient(hwnd, &mut point).as_bool() {
        return false;
    }
    let factor = 96. / GetDpiForWindow(hwnd).max(96) as f32;
    views(state).is_some_and(|v| {
        v.hit([point.x as f32 * factor, point.y as f32 * factor])
            .is_some()
    })
}
unsafe fn refresh(hwnd: HWND, state: &State) -> Result<()> {
    super::extension_visuals::invalidate(hwnd);
    super::pin_window::stationary::refresh(hwnd, state)
}
unsafe fn cancel(hwnd: HWND, state: &State, completed: bool) -> bool {
    let raw = state
        .canvas
        .borrow()
        .extension_views
        .as_ref()
        .map(|b| b.shared());
    let captured = raw.is_some_and(|v| v.cancel(completed));
    if captured && GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    captured
}
unsafe fn modifiers() -> Modifiers {
    let held = |k: VIRTUAL_KEY| GetKeyState(k.0 as i32) < 0;
    Modifiers {
        ctrl_key: held(VK_CONTROL),
        alt_key: held(VK_MENU),
        shift_key: held(VK_SHIFT),
        meta_key: held(VK_LWIN) || held(VK_RWIN),
    }
}
unsafe fn copy_with(
    hwnd: HWND,
    state: &State,
    v: &Rc<Views>,
    text: &str,
    selection: bool,
    publish: impl FnOnce(&str, &dyn Fn() -> bool) -> Result<()>,
) -> Result<()> {
    // 复制前和获得剪贴板锁后都复核来源；同步 Win32 调用期间不持模型/布局借用。
    let current = || {
        state.lifetime.owns(hwnd)
            && same(state, v)
            && (!selection
                || v.selected_text()
                    .is_ok_and(|selected| selected.as_deref() == Some(text)))
    };
    if current() {
        if let Err(error) = publish(text, &current) {
            eprintln!("native-extension-copy: {error}");
            if current() {
                super::extension_notices::show(
                    hwnd,
                    state,
                    None,
                    "复制失败",
                    "请再次点击文本重试。",
                )?;
            }
        }
    }
    Ok(())
}
unsafe fn copy(
    hwnd: HWND,
    state: &State,
    v: &Rc<Views>,
    text: &str,
    selection: bool,
) -> Result<()> {
    copy_with(hwnd, state, v, text, selection, |text, current| {
        super::clipboard::copy_extension_text_checked(hwnd, text, current)
    })
}
pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !matches!(
        msg,
        WM_LBUTTONDOWN
            | WM_LBUTTONDBLCLK
            | WM_LBUTTONUP
            | WM_MOUSEMOVE
            | WM_SETCURSOR
            | WM_KEYDOWN
            | WM_KEYUP
            | WM_CHAR
            | WM_CANCELMODE
            | WM_KILLFOCUS
            | WM_CAPTURECHANGED
            | WM_SHOWWINDOW
            | WM_DESTROY
    ) {
        return None;
    }
    if matches!(
        msg,
        WM_CANCELMODE | WM_KILLFOCUS | WM_CAPTURECHANGED | WM_DESTROY
    ) || msg == WM_SHOWWINDOW && wp.0 == 0
    {
        let changed = cancel(hwnd, state, msg == WM_DESTROY || msg == WM_SHOWWINDOW);
        if changed {
            if let Err(e) = refresh(hwnd, state) {
                eprintln!("native-extension-selection: {e}");
            }
        }
        return None;
    }
    let v = match views(state) {
        Some(v) => v,
        None => {
            cancel(hwnd, state, true);
            return None;
        }
    };
    let result = message(hwnd, state, &v, msg, wp, lp);
    match result {
        Ok(true) => Some(LRESULT(if msg == WM_SETCURSOR { 1 } else { 0 })),
        Ok(false) => None,
        Err(e) => {
            eprintln!("native-extension-input: {e}");
            cancel(hwnd, state, true);
            Some(LRESULT(0))
        }
    }
}
unsafe fn message(
    hwnd: HWND,
    state: &State,
    v: &Rc<Views>,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Result<bool> {
    let point = logical(hwnd, lp);
    match msg {
        WM_KEYDOWN | WM_KEYUP | WM_CHAR => {
            let mods = modifiers();
            if mods.ctrl_key
                && !mods.alt_key
                && !mods.meta_key
                && matches!((msg, wp.0), (WM_KEYDOWN | WM_KEYUP, 0x43) | (WM_CHAR, 3))
                && GetFocus() == hwnd
            {
                if let Some(text) = v.selected_text()? {
                    if msg == WM_KEYDOWN && lp.0 & (1 << 30) == 0 {
                        copy(hwnd, state, v, &text, true)?;
                    }
                    return Ok(true);
                }
            }
            if msg == WM_KEYDOWN && wp.0 == VK_ESCAPE.0 as usize {
                cancel(hwnd, state, true);
                refresh(hwnd, state)?;
            }
        }
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
            if v.begin(point, msg == WM_LBUTTONDBLCLK)? {
                // 不重新激活/聚焦文字，避免破坏已开始的原生选区。
                SetCapture(hwnd);
                if GetCapture() != hwnd || !same(state, v) {
                    cancel(hwnd, state, true);
                }
                refresh(hwnd, state)?;
                return Ok(true);
            }
            refresh(hwnd, state)?;
        }
        WM_MOUSEMOVE | WM_LBUTTONUP if v.dragging() => {
            if GetCapture() != hwnd {
                cancel(hwnd, state, false);
                return Ok(true);
            }
            if msg == WM_MOUSEMOVE {
                v.update(point)?;
            } else {
                let outcome = v.finish(point, modifiers())?;
                // 已完成选区先结束手势，再归还捕获，WM_CAPTURECHANGED 不清已完成范围。
                let _ = ReleaseCapture();
                if same(state, v) {
                    match outcome {
                        Outcome::Copy(text) => copy(hwnd, state, v, &text, false)?,
                        Outcome::Command { command, input } => {
                            super::extension_commands::request_input(hwnd, state, &command, input)?
                        }
                        Outcome::None => (),
                    }
                }
            }
            refresh(hwnd, state)?;
            return Ok(true);
        }
        WM_SETCURSOR if lp.0 as u16 == HTCLIENT as u16 => {
            let mut p = POINT::default();
            if GetCursorPos(&mut p).is_ok() && ScreenToClient(hwnd, &mut p).as_bool() {
                let factor = 96. / GetDpiForWindow(hwnd).max(96) as f32;
                if v.hit([p.x as f32 * factor, p.y as f32 * factor]).is_some() {
                    SetCursor(Some(LoadCursorW(None, IDC_HAND)?));
                    return Ok(true);
                }
            }
        }
        _ => (),
    }
    Ok(false)
}
#[cfg(test)]
#[path = "extension_input_tests.rs"]
mod tests;
