//! Precisely distinguish mainline pin zoom, opacity, and annotation scaling; never misroute Ctrl+Alt to window-drag presentation.
use crate::{
    desktop::{self, State},
    viewport::Point,
};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*},
};

pub(super) unsafe fn dispatch(hwnd: HWND, state: &State, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if state.canvas.borrow().blocks_wheel() {
        return LRESULT(0);
    }
    if state.canvas.borrow().pinned {
        let control = GetKeyState(VK_CONTROL.0 as i32) < 0;
        let alt = GetKeyState(VK_MENU.0 as i32) < 0;
        let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
        let meta = [VK_LWIN, VK_RWIN]
            .iter()
            .any(|key| GetKeyState(key.0 as i32) < 0);
        match (control, alt, shift, meta) {
            (true, true, own, false) => {
                desktop::resize_input::wheel::dispatch(hwnd, state, (wp.0 >> 16) as i16, own)
            }
            (true, false, false, false) => desktop::pin_window::wheel(hwnd, state, wp, lp),
            (false, true, false, false) => desktop::pin_window::opacity_wheel(hwnd, state, wp),
            _ => (),
        }
        return LRESULT(0);
    }
    let mut point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    if ScreenToClient(hwnd, &mut point).as_bool() {
        let dpi = GetDpiForWindow(hwnd).max(1) as f32 / 96.0;
        let steps = (wp.0 >> 16) as i16 as f32 / 120.0;
        if state.canvas.borrow_mut().viewport.zoom_at(
            Point {
                x: point.x as f32 / dpi,
                y: point.y as f32 / dpi,
            },
            steps,
        ) {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
    LRESULT(0)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "procedure_wheel_window_tests.rs"]
mod window_tests;
