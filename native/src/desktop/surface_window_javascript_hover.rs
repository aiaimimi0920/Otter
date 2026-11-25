//! Button-free hover samples only current-HWND hits, without stealing focus/capture, sharing leave tracking with original mouse visuals.
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
    // The five mouse-button bits in WM_MOUSEMOVE; Ctrl/Shift are modifiers, still sampled by the original modifier owner.
    if message != WM_MOUSELEAVE && wp.0 & 0x73 != 0 {
        return;
    }
    if !hover_eligible(hwnd, state) {
        cancel(state);
        return;
    }
    // Hover must not steal capture from native controls/selection/scrollbars; Canvas up already released capture before this call.
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
        // Canvas up only resumes existing hover; pure dragging must not fabricate additional input.
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
