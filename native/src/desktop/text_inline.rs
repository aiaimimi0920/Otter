//! 钉图就地输入会话；草稿仅在 EDIT 中，退出时复用 Pending 的身份核验和单次撤销事务。
use super::{Pending, State};
use crate::{desktop, text_annotation::TextAnnotation, viewport::Point};
use std::rc::Rc;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "text_inline_messages.rs"]
mod messages;
#[path = "text_inline_paint.rs"]
mod paint;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "text_inline_tests.rs"]
mod tests;
#[path = "text_inline_window.rs"]
mod window;

pub(in crate::desktop) struct Session {
    pending: Pending,
    ui: window::Window,
}
pub(in crate::desktop) unsafe fn begin(hwnd: HWND, state: &State, pending: Pending) -> Result<()> {
    if state.file_dialog.get() || state.group_locked.get() || state.session_end.paused() {
        return Ok(());
    }
    finish(hwnd, state, true)?;
    let initial = pending.text.as_ref().map_or("", TextAnnotation::content);
    let scale = state.canvas.borrow().viewport.scale * GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
    let ui = window::Window::new(hwnd, initial, &pending.font, pending.style, scale)?;
    let session = Rc::new(Session { pending, ui });
    *state.inline_text.borrow_mut() = Some(session.clone());
    state.canvas.borrow_mut().inline_text_hidden = session.pending.index;
    let _ = InvalidateRect(Some(hwnd), None, false);
    let result = resize(hwnd, state, &session).and_then(|()| session.ui.show());
    if result.is_err() {
        state.inline_text.borrow_mut().take();
        state.canvas.borrow_mut().inline_text_hidden = None;
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    result
}
unsafe fn resize(hwnd: HWND, state: &State, session: &Session) -> Result<()> {
    let content = session.ui.text()?;
    let font = &session.pending.font;
    let measure = TextAnnotation::restored(
        if content.is_empty() { "M" } else { &content },
        font.family(),
        font.font_size(),
        font.bold(),
    )
    .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid inline text"))?;
    let measured = desktop::annotation_text::measure(&measure)?;
    let (viewport, anchor) = {
        let canvas = state.canvas.borrow();
        let anchor = session
            .pending
            .index
            .and_then(|i| canvas.editor.elements().get(i))
            .map_or(session.pending.point, |stroke| {
                crate::rotation::rotate(
                    stroke.points[0],
                    crate::rotation::center(&stroke.points),
                    stroke.rotation,
                )
            });
        (canvas.viewport, anchor)
    };
    let dpi = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
    let scale = viewport.scale * dpi;
    let mut origin = POINT {
        x: ((anchor.x * viewport.scale + viewport.origin.x) * dpi).round() as i32,
        y: ((anchor.y * viewport.scale + viewport.origin.y) * dpi).round() as i32,
    };
    if !ClientToScreen(hwnd, &mut origin).as_bool() {
        return Err(Error::from_win32());
    }
    // 初始留出八字宽空白；输入后仅由内容扩展，极端字号/长文本由系统 EDIT 滚动。
    let initial = if session.pending.index.is_none() {
        font.font_size() * 8.0 + 8.0
    } else {
        0.0
    };
    let size = Point {
        x: (measured.x.max(initial) * scale + 2.0).clamp(24.0, 8192.0),
        y: (measured.y * scale + 2.0).clamp(20.0, 4096.0),
    };
    session.ui.place(origin, size)
}
pub(in crate::desktop) unsafe fn finish(hwnd: HWND, state: &State, commit: bool) -> Result<()> {
    // 先移出会话：读控件、关窗口、转移焦点均可重入，不能持有 RefCell 借用。
    let session = state.inline_text.borrow_mut().take();
    let Some(session) = session else {
        return Ok(());
    };
    // 让标准 EDIT 按其 IME 策略完成组合串，再读取最终值；会话已移出，不会递归提交。
    if session.ui.contains(GetFocus()) && state.lifetime.owns(hwnd) {
        let _ = SetFocus(Some(hwnd));
    }
    let result = if commit && state.lifetime.owns(hwnd) && !state.session_end.paused() {
        session
            .ui
            .text()
            .and_then(|content| super::commit_content(state, &session.pending, &content))
    } else {
        Ok(())
    };
    if let Err(error) = result {
        *state.inline_text.borrow_mut() = Some(session);
        return Err(error); // 不把读失败或提交失败伪装成成功，不丢草稿。
    }
    session.ui.hide();
    state.canvas.borrow_mut().inline_text_hidden = None;
    drop(session);
    if state.lifetime.owns(hwnd) {
        desktop::hit_region::update(hwnd, state)?;
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Ok(())
}
fn report(result: Result<()>) -> bool {
    if let Err(error) = result {
        eprintln!("native-inline-text: {error}");
        true
    } else {
        false
    }
}
pub(in crate::desktop) unsafe fn before_message(hwnd: HWND, state: &State, message: &MSG) -> bool {
    let session = state.inline_text.borrow().clone();
    let Some(session) = session else {
        return false;
    };
    if session.ui.forward_click(hwnd, message) {
        return true;
    }
    let outside = !session.ui.contains(message.hwnd);
    let click = matches!(
        message.message,
        WM_LBUTTONDOWN
            | WM_LBUTTONDBLCLK
            | WM_RBUTTONDOWN
            | WM_MBUTTONDOWN
            | WM_NCLBUTTONDOWN
            | WM_NCRBUTTONDOWN
    );
    let save = message.message == WM_KEYDOWN
        && message.wParam.0 == 0x53
        && GetKeyState(VK_CONTROL.0 as i32) < 0;
    if outside && click || save {
        let failed = report(finish(hwnd, state, true));
        if save && !outside && !failed {
            // 原消息指向即将销毁的 EDIT，保存快捷键必须显式交还宿主而非丢到旧句柄。
            SendMessageW(hwnd, WM_KEYDOWN, Some(message.wParam), Some(message.lParam));
        }
        failed || save && !outside
    } else {
        false
    }
}
pub(in crate::desktop) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    let session = state.inline_text.borrow().clone()?;
    if msg == WM_MOUSEACTIVATE {
        let position = GetMessagePos();
        let point = POINT {
            x: position as i16 as i32,
            y: (position >> 16) as i16 as i32,
        };
        if session.ui.keyed_edit_point(point).is_some() {
            return Some(LRESULT(MA_NOACTIVATE as isize));
        }
    }
    if matches!(msg, messages::FINISH | messages::CHANGE | messages::CANCEL) {
        if session.ui.matches(wp, lp) {
            let result = if msg != messages::CHANGE {
                finish(hwnd, state, msg == messages::FINISH)
            } else {
                resize(hwnd, state, &session)
            };
            report(result);
        }
        return Some(LRESULT(0));
    }
    if msg == WM_KEYDOWN && wp.0 == VK_ESCAPE.0 as usize {
        // 草稿失败后焦点可能已回宿主；取消不能落入空闲 Esc 的删除钉图路径。
        report(finish(hwnd, state, false));
        return Some(LRESULT(0));
    }
    if matches!(msg, WM_DESTROY | WM_NCDESTROY) {
        let retired = state.inline_text.borrow_mut().take();
        if let Ok(mut canvas) = state.canvas.try_borrow_mut() {
            canvas.inline_text_hidden = None;
        }
        drop(retired); // 宿主正在销毁，不重绘或把焦点交回即将失效的 HWND。
    } else if matches!(
        msg,
        WM_LBUTTONDOWN
            | WM_LBUTTONDBLCLK
            | WM_CONTEXTMENU
            | WM_KEYDOWN
            | WM_MOUSEWHEEL
            | WM_MOUSEHWHEEL
            | WM_CLOSE
            | WM_SIZE
            | WM_DPICHANGED
            | WM_HOTKEY
            | WM_COMMAND
            | desktop::tool_panel::EXECUTE
    ) {
        if report(finish(hwnd, state, true)) {
            return Some(LRESULT(0));
        }
    } else if msg == WM_WINDOWPOSCHANGING && lp.0 != 0 {
        let pos = &*(lp.0 as *const WINDOWPOS);
        if (!pos.flags.contains(SWP_NOMOVE | SWP_NOSIZE) || pos.flags.contains(SWP_HIDEWINDOW))
            && report(finish(hwnd, state, true))
        {
            return Some(LRESULT(0));
        }
    }
    None
}
