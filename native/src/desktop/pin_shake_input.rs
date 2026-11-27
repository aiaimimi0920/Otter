//! Only mouse-initiated caption movement feeds the detector. Ordinary
//! WM_WINDOWPOSCHANGED (layout, DPI and keyboard moves) never arms it.
use super::State;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::Win32::{
    Foundation::*,
    UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
#[derive(Default)]
pub(super) struct Signals {
    pub count: Cell<usize>,
    pub restore: Cell<bool>,
}
#[derive(Default)]
pub(super) struct Input {
    pub signals: Rc<Signals>,
    pub pending: Cell<bool>,
    pub hidden: Cell<bool>,
    pub panel: Cell<bool>,
    gesture: RefCell<crate::pin_shake_gesture::Shake>,
    armed: Cell<bool>,
}
pub(super) unsafe fn observe(hwnd: HWND, s: &State, msg: u32, wp: WPARAM) {
    match msg {
        WM_NCLBUTTONDOWN
            if wp.0 as u32 == HTCAPTION
                && GetKeyState(VK_LBUTTON.0 as i32) < 0
                && s.canvas.try_borrow().is_ok_and(|c| c.pinned)
                && !s.group_locked.get()
                && !s.is_art() =>
        {
            let mut p = POINT::default();
            if GetCursorPos(&mut p).is_ok() {
                s.shake.gesture.borrow_mut().begin(
                    p.x,
                    GetMessageTime() as u32,
                    GetDpiForWindow(hwnd),
                );
                s.shake.armed.set(true);
            }
        }
        WM_MOVING if s.shake.armed.get() && GetKeyState(VK_LBUTTON.0 as i32) < 0 => {
            let mut p = POINT::default();
            if GetCursorPos(&mut p).is_ok()
                && s.shake
                    .gesture
                    .borrow_mut()
                    .sample(p.x, GetMessageTime() as u32)
            {
                s.shake.pending.set(!s.shake.pending.get());
                let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            }
        }
        WM_EXITSIZEMOVE | WM_CANCELMODE | WM_NCDESTROY | WM_LBUTTONUP | WM_NCLBUTTONUP => {
            s.shake.armed.set(false);
            s.shake.gesture.borrow_mut().end();
        }
        _ => {}
    }
}
