//! 单槽滚动条捕获；开始前废弃动作候选，拖动期间不排队点击或网络动作。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
pub(super) fn clear(state: &State) {
    state.scroll_drag.borrow_mut().take();
    let repeat = state.scroll_repeat.borrow_mut().take();
    drop(repeat);
}
fn point(lp: LPARAM) -> [i32; 2] {
    [
        lp.0 as u16 as i16 as i32,
        (lp.0 as usize >> 16) as u16 as i16 as i32,
    ]
}
pub(super) unsafe fn message(hwnd: HWND, state: &State, message: u32, lp: LPARAM) -> bool {
    if matches!(message, WM_CAPTURECHANGED | WM_CANCELMODE | WM_KILLFOCUS) {
        clear(state);
        return false;
    }
    if state.closed.get() || !state.local_edit.get() {
        return false;
    }
    let pixels = point(lp);
    if state.scroll_repeat.borrow().is_some() {
        if message == WM_LBUTTONUP {
            pointer::cancel(hwnd, state);
            return true;
        }
        if message == WM_MOUSEMOVE {
            if GetCapture() != hwnd {
                pointer::cancel(hwnd, state);
            } else if let Some(repeat) = state.scroll_repeat.borrow_mut().as_mut() {
                repeat.point = pixels;
            }
            return true;
        }
    }
    let drag = state.scroll_drag.borrow().clone();
    if let Some(drag) = drag {
        if matches!(message, WM_MOUSEMOVE | WM_LBUTTONUP) {
            if GetCapture() == hwnd {
                let result = state.run(|host| host.drag_scrollbar(&drag, pixels));
                if let Err(error) = result {
                    *state.error.borrow_mut() = Some(error);
                    pointer::invalidate(hwnd, state);
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            } else {
                clear(state);
            }
            if message == WM_LBUTTONUP {
                clear(state);
                if GetCapture() == hwnd {
                    let _ = ReleaseCapture();
                }
            }
            return true;
        }
    }
    // 没有条拖动时，普通内容的 move/up 必须回到 pointer 完成取消与释放捕获。
    if !matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) {
        return false;
    }
    let mut hit = None;
    let result = state.run(|host| {
        hit = host.scrollbar(pixels)?;
        Ok(())
    });
    match result {
        Ok(true) if !state.closed.get() && state.local_edit.get() => {}
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            pointer::invalidate(hwnd, state);
            return true;
        }
        _ => return true,
    }
    let Some((bar, logical)) = hit else {
        return false;
    };
    if matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) {
        pointer::invalidate(hwnd, state);
        // 清理旧 capture 可能同步重入关闭/权限撤销，不能随后复活拖动。
        if state.closed.get() || !state.local_edit.get() {
            return true;
        }
        if let Some(drag) = bar.drag(logical) {
            // 不跨 SetCapture 的同步重入持有 RefCell 借用。
            *state.scroll_drag.borrow_mut() = Some(drag);
            let _ = SetCapture(hwnd);
            if GetCapture() != hwnd
                || state.closed.get()
                || !state.local_edit.get()
                || state.scroll_drag.borrow().is_none()
            {
                pointer::cancel(hwnd, state);
            }
        } else if let Err(error) = repeat::start(hwnd, state, &bar, logical, pixels) {
            *state.error.borrow_mut() = Some(error);
            pointer::invalidate(hwnd, state);
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    true
}
