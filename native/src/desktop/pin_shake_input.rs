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
    pub active_origin: Cell<Option<(HWND, isize)>>,
    pub sync_chrome: Cell<bool>,
    pub peers: RefCell<Vec<(HWND, HWND, isize)>>,
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

unsafe fn immediate_toggle(origin_hwnd: HWND, origin_state: &State) {
    if !super::tray::recovery_installed(origin_state) {
        return;
    }
    let peers = origin_state.shake.signals.peers.borrow().clone();
    if origin_state.shake.signals.count.get() > 0 {
        origin_state.shake.signals.count.set(0);
        origin_state.shake.signals.active_origin.set(None);
        for (peer_hwnd, peer_panel, marker) in peers {
            if !IsWindow(Some(peer_hwnd)).as_bool()
                || GetWindowLongPtrW(peer_hwnd, GWLP_USERDATA) != marker
            {
                continue;
            }
            let ps = &*(marker as *const State);
            if !ps.shake.hidden.replace(false) {
                continue;
            }
            let show_panel = ps.shake.panel.replace(false);
            if ps.group_hidden.get() || super::tray::is_hidden(ps) {
                continue;
            }
            let _ = ShowWindow(peer_hwnd, SW_SHOWNOACTIVATE);
            if show_panel && IsWindow(Some(peer_panel)).as_bool() {
                let _ = ShowWindow(peer_panel, SW_SHOWNOACTIVATE);
                if neuro_mini_ui::is_toolbar(peer_panel) {
                    if let Some(props) = neuro_mini_ui::property_window(peer_panel) {
                        let _ = ShowWindow(props, SW_SHOWNOACTIVATE);
                    }
                }
            }
        }
        origin_state.shake.signals.sync_chrome.set(true);
        return;
    }
    let mut hidden_count = 0usize;
    for (peer_hwnd, peer_panel, marker) in peers {
        if peer_hwnd == origin_hwnd
            || !IsWindow(Some(peer_hwnd)).as_bool()
            || GetWindowLongPtrW(peer_hwnd, GWLP_USERDATA) != marker
        {
            continue;
        }
        let ps = &*(marker as *const State);
        if !ps.canvas.try_borrow().is_ok_and(|c| c.pinned)
            || ps.group_hidden.get()
            || super::tray::is_hidden(ps)
            || !IsWindowVisible(peer_hwnd).as_bool()
        {
            continue;
        }
        if let Some(settings) = super::pin_settings::window(ps) {
            let _ = ShowWindow(settings, SW_HIDE);
        }
        ps.shake.panel.set(IsWindowVisible(peer_panel).as_bool());
        if neuro_mini_ui::is_toolbar(peer_panel) {
            if let Some(props) = neuro_mini_ui::property_window(peer_panel) {
                let _ = ShowWindow(props, SW_HIDE);
            }
        }
        ps.shake.hidden.set(true);
        let _ = ShowWindow(peer_panel, SW_HIDE);
        let _ = super::application::chrome::hide_pin(peer_hwnd, ps);
        let _ = ShowWindow(peer_hwnd, SW_HIDE);
        hidden_count += 1;
    }
    if hidden_count > 0 {
        origin_state.shake.signals.count.set(hidden_count);
        origin_state
            .shake
            .signals
            .active_origin
            .set(Some((origin_hwnd, origin_state as *const State as isize)));
        origin_state.shake.signals.sync_chrome.set(true);
    }
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
                if !s.shake.signals.peers.borrow().is_empty() {
                    immediate_toggle(hwnd, s);
                } else {
                    s.shake.pending.set(!s.shake.pending.get());
                }
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
