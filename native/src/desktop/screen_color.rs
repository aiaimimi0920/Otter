//! 显式屏幕取色会话；事件驱动单像素采样，临时 hook 随会话释放，无空闲定时器。
use super::State;
use crate::{
    editor::{DocumentStamp, Tool},
    stroke_style::{Color, StrokeStyle},
};
use std::{cell::Cell, rc::Rc};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "screen_color_hook.rs"]
mod hook;
#[path = "screen_color_preview.rs"]
mod preview;
pub(super) const STROKE: usize = 0x1064;
pub(super) const FILL: usize = 0x1065;
struct Modal(Rc<Cell<bool>>);
impl Drop for Modal {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
pub(super) struct Session {
    hook: Option<hook::Hook>,
    stamp: DocumentStamp,
    style: StrokeStyle,
    tool: Tool,
    fill: bool,
    preview: super::Window,
    _modal: Modal,
}
pub(super) unsafe fn append(menu: HMENU) -> Result<()> {
    AppendMenuW(menu, MF_STRING, STROKE, w!("屏幕取色：画笔 / 边框"))?;
    AppendMenuW(menu, MF_STRING, FILL, w!("屏幕取色：填充"))
}
pub(super) unsafe fn start(hwnd: HWND, state: &State, fill: bool) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool()
        || state.modals.blocked()
        || state.file_dialog.get()
        || state.group_locked.get()
        || state.canvas.borrow().minified.is_some()
    {
        return Ok(());
    }
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    let cursor = LoadCursorW(None, IDC_CROSS)?;
    let mut session = prepare(hwnd, state, fill)?;
    // 用户显式取色才激活宿主，以接收 Escape；跨进程鼠标输入由临时 hook 转发。
    let _ = SetForegroundWindow(hwnd);
    if GetForegroundWindow() != hwnd {
        return Err(Error::new(
            E_FAIL,
            "Screen color picker requires foreground ownership",
        ));
    }
    session.hook = Some(hook::Hook::install(hwnd)?);
    *state.screen_color.borrow_mut() = Some(session);
    SetCursor(Some(cursor));
    Ok(())
}
unsafe fn prepare(hwnd: HWND, state: &State, fill: bool) -> Result<Session> {
    if state.file_dialog.replace(true) {
        return Err(Error::new(E_FAIL, "Color picker modal already active"));
    }
    let modal = Modal(state.file_dialog.clone());
    let window = preview::create(hwnd)?;
    let canvas = state.canvas.borrow();
    Ok(Session {
        hook: None,
        stamp: canvas.editor.document_stamp(),
        style: canvas.editor.style(),
        tool: canvas.tool,
        fill,
        preview: window,
        _modal: modal,
    })
}
pub(super) unsafe fn dispatch(hwnd: HWND, state: &State, msg: u32, wp: WPARAM) -> Option<LRESULT> {
    if state.screen_color.borrow().is_none() {
        return None;
    }
    if matches!(msg, hook::MOVED | hook::COMMIT | hook::CANCEL) && !hook::is_current(wp.0) {
        return Some(LRESULT(0));
    }
    match msg {
        WM_CLOSE | WM_DESTROY | WM_CANCELMODE | WM_KILLFOCUS | WM_CAPTURECHANGED => {
            finish(hwnd, state, None);
            return None;
        }
        WM_KEYDOWN if wp.0 == VK_ESCAPE.0 as usize => finish(hwnd, state, None),
        WM_RBUTTONDOWN => finish(hwnd, state, None),
        hook::COMMIT => {
            let popup = state
                .screen_color
                .borrow()
                .as_ref()
                .map(|session| session.preview.0);
            finish(hwnd, state, preview::sample(hook::commit_point(), popup));
        }
        hook::CANCEL => finish(hwnd, state, None),
        hook::MOVED => {
            let point = hook::take_move();
            let popup = state
                .screen_color
                .borrow()
                .as_ref()
                .map(|session| session.preview.0);
            if let Some(color) = preview::sample(point, popup) {
                if let Some(popup) = popup {
                    preview::show(popup, point, color);
                }
            }
        }
        WM_SETCURSOR => {
            if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                SetCursor(Some(cursor));
            }
        }
        WM_COMMAND | WM_CONTEXTMENU | WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP
        | WM_LBUTTONDBLCLK | WM_RBUTTONUP | WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_KEYDOWN
        | WM_KEYUP => (),
        _ => return None,
    }
    Some(LRESULT(1))
}
unsafe fn finish(hwnd: HWND, state: &State, color: Option<Color>) {
    let session = state.screen_color.borrow_mut().take();
    let Some(mut session) = session else {
        return;
    };
    if session.hook.take().is_some() && GetForegroundWindow() == hwnd {
        if let Ok(cursor) = LoadCursorW(None, IDC_ARROW) {
            SetCursor(Some(cursor));
        }
    }
    if IsWindow(Some(hwnd)).as_bool() && !state.group_locked.get() {
        if let Some(color) = color {
            let mut canvas = state.canvas.borrow_mut();
            if session.stamp.matches(&canvas.editor)
                && session.tool == canvas.tool
                && session.style == canvas.editor.style()
            {
                canvas.editor.set_style(if session.fill {
                    session.style.with_fill(Some(color))
                } else {
                    session.style.with_color(color)
                });
                state.capture_history.borrow_mut().color(color);
            }
        }
    }
    drop(session);
}

#[cfg(test)]
#[path = "screen_color_tests.rs"]
mod tests;
