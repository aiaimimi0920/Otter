//! Native pointer enter/leave edges; state uses one bit in existing subclass ref-data and owns no timer.
use super::*;
use windows::Win32::UI::{Controls::WM_MOUSELEAVE, Input::KeyboardAndMouse::*};
pub(super) unsafe fn cancel_tracking(hwnd: HWND) {
    let mut event = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE | TME_CANCEL,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    let _ = TrackMouseEvent(&mut event);
}
unsafe fn clear(hwnd: HWND) {
    if messages::hovered(hwnd) {
        messages::set_hover(hwnd, false);
        cancel_tracking(hwnd);
    }
}
pub(super) unsafe fn update(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) {
    if message == WM_MOUSELEAVE
        || message == WM_CANCELMODE
        || (matches!(message, WM_ENABLE | WM_SHOWWINDOW) && wp.0 == 0)
    {
        clear(hwnd);
        return;
    }
    if message != WM_MOUSEMOVE {
        return;
    }
    let point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    let mut rect = RECT::default();
    if !IsWindowEnabled(hwnd).as_bool()
        || GetClientRect(hwnd, &mut rect).is_err()
        || !PtInRect(&rect, point).as_bool()
    {
        clear(hwnd);
        return;
    }
    if messages::hovered(hwnd) {
        return;
    }
    // Record state before requesting system leave tracking, preventing synchronous/later leave notifications from being overwritten with true.
    if !messages::set_hover(hwnd, true) {
        return;
    }
    let mut event = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    if TrackMouseEvent(&mut event).is_err() {
        messages::set_hover(hwnd, false);
    }
}
