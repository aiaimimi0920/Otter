//! 文字拖选与普通点击共用父捕获；完成选区不进入远端动作或数值编辑队列。
use super::*;
#[path = "surface_window_text_autoscroll.rs"]
pub(super) mod autoscroll;
#[path = "surface_window_text_copy.rs"]
mod copy;
#[path = "surface_window_text_key.rs"]
mod key;
pub(in crate::desktop::surface_window) use copy::message as copy_message;
pub(in crate::desktop::surface_window) use key::message as text_key_message;

pub(super) fn cancel(state: &State, completed: bool) {
    state.pointer.borrow_mut().selection_owner = None;
    state.pointer.borrow_mut().selection_scroll = None;
    // Host 被同步绘制/控件回调暂时取出时，撤销也必须在归还前落地。
    let mode = if completed { 2 } else { 1 };
    let pending = state.pointer.borrow().selection_cancel.max(mode);
    state.pointer.borrow_mut().selection_cancel = pending;
    if let Err(error) = state.run(|_| Ok(())) {
        *state.error.borrow_mut() = Some(error);
    }
}
pub(super) fn reconcile(state: &State, host: &mut Host) {
    let mode = state
        .pointer
        .try_borrow_mut()
        .map(|mut pointer| std::mem::take(&mut pointer.selection_cancel))
        .unwrap_or(0);
    if mode != 0 {
        host.cancel_text_selection(mode == 2);
    }
}
fn apply(
    hwnd: HWND,
    state: &State,
    f: impl FnOnce(&mut crate::desktop::surface_host::Host) -> Result<bool>,
) -> bool {
    let mut value = false;
    match state.run(|host| {
        value = f(host)?;
        Ok(())
    }) {
        Ok(true) if !state.closed.get() => value,
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            cancel(state, true);
            super::cancel(hwnd, state);
            false
        }
        _ => {
            cancel(state, true);
            super::cancel(hwnd, state);
            false
        }
    }
}
pub(super) unsafe fn message(hwnd: HWND, state: &State, message: u32, lp: LPARAM) -> bool {
    if !state.local_edit.get() {
        return false;
    }
    let position = point(lp);
    if matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) {
        if state.pointer.borrow().pending.is_some() {
            return false;
        }
        cancel(state, true);
        let Some(node) = hit(state, position) else {
            return false;
        };
        // 切焦会撤销旧选区，必须在它之后开始新选择。
        let selectable = apply(hwnd, state, |host| {
            Ok(host
                .presentation()
                .text()
                .items()
                .iter()
                .any(|item| item.node_id() == node && item.selectable()))
        });
        if !selectable {
            return false;
        }
        keyboard::focus_node(hwnd, state, &node);
        if !state.local_edit.get() || state.closed.get() {
            return true;
        }
        if !apply(hwnd, state, |host| {
            if message == WM_LBUTTONDBLCLK {
                host.begin_word_selection(position)
            } else {
                host.begin_text_selection(position)
            }
        }) {
            // A failed word operation must not turn into an ancestor action on mouse-up.
            return message == WM_LBUTTONDBLCLK;
        }
        if !state.local_edit.get()
            || !state
                .host
                .borrow()
                .as_ref()
                .is_some_and(|host| host.presentation().text_selection_dragging())
        {
            return true;
        }
        state.pointer.borrow_mut().selection_owner = Some(Rc::new(()));
        state.pointer.borrow_mut().pressed = Some(Press {
            point: position,
            node,
        });
        let _ = SetCapture(hwnd);
        if GetCapture() != hwnd {
            super::cancel(hwnd, state);
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
        return true;
    }
    let dragging = apply(hwnd, state, |host| {
        Ok(host.presentation().text_selection_dragging())
    });
    if !dragging {
        return false;
    }
    if GetCapture() != hwnd {
        super::cancel(hwnd, state);
        return false;
    }
    match message {
        WM_MOUSEMOVE => {
            if !apply(hwnd, state, |host| {
                host.update_text_selection(position, false)
            }) {
                super::cancel(hwnd, state);
            } else {
                autoscroll::update(hwnd, state, position);
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
            true
        }
        WM_LBUTTONUP => {
            state.pointer.borrow_mut().selection_owner = None;
            state.pointer.borrow_mut().selection_scroll = None;
            let selected = apply(hwnd, state, |host| {
                host.update_text_selection(position, true)
            });
            let _ = InvalidateRect(Some(hwnd), None, false);
            if !selected {
                return false;
            } // collapsed 选择仍交回原单击合同。
            state.pointer.borrow_mut().pressed = None;
            let _ = ReleaseCapture();
            true
        }
        _ => false,
    }
}
#[cfg(test)]
#[path = "surface_window_text_selection_tests.rs"]
mod tests;
