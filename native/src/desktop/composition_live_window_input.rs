//! Input adapter for Live HWNDs; enqueue only, without permission queries/cross-window delivery. Revocation signals can bypass nested message pumps.
use super::*;
use crate::{live_input::Event, live_input_worker::StopSignal};
use std::collections::VecDeque;
use windows::Win32::UI::Controls::{WM_MOUSEHOVER, WM_MOUSELEAVE};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
#[path = "composition_live_input_control.rs"]
mod control;
use control::menu_status;
#[cfg(test)]
#[path = "composition_live_hover_tests.rs"]
mod hover_tests;
#[path = "composition_live_ime.rs"]
mod ime;
#[path = "composition_live_pointer_tracking.rs"]
mod pointer_tracking;
#[path = "composition_live_popup_input.rs"]
mod popup;
#[cfg(test)]
#[path = "composition_live_input_viewport_tests.rs"]
mod viewport_tests;
pub(super) use popup::Focus as PopupFocus;
#[path = "composition_live_input_queue.rs"]
mod queue;
pub(super) const TOGGLE: usize = 0x1ff0;
#[derive(Default)]
pub(super) struct Control {
    pub(super) hover: Option<crate::live_hover_owner::Owner>,
    tracking: pointer_tracking::Tracking,
    ime: ime::Anchor,
    allowed: bool,
    desired: bool,
    remote: bool,
    enabled: bool,
    pending: bool,
    buttons: u8,
    pointer: Option<[f64; 2]>,
    held_keys: [u64; 4],
    request: Option<bool>,
    events: VecDeque<Event>,
    stop: Option<StopSignal>,
    shown: Option<(bool, bool)>,
    failed: bool,
    notice: bool,
}
impl LiveWindow {
    pub(crate) fn block_local_input(&self, remote: bool) {
        self.state.input.borrow_mut().remote = remote;
        if self.is_alive() {
            unsafe {
                menu_status(self.hwnd, &self.state, false, false);
                wheel::sync(self.hwnd, &self.state);
            }
        }
    }
    pub(in crate::desktop) fn take_input_notice(&self) -> bool {
        std::mem::take(&mut self.state.input.borrow_mut().notice)
    }
    pub(crate) fn allow_input(&self) -> Result<()> {
        self.state.input.borrow_mut().allowed = true;
        unsafe {
            let _ = EnableMenuItem(
                GetSystemMenu(self.hwnd, false),
                TOGGLE as u32,
                MF_BYCOMMAND | MF_ENABLED,
            );
            let _ = EnableMenuItem(
                GetSystemMenu(self.hwnd, false),
                super::publication::PUBLISH_CONTROL as u32,
                MF_BYCOMMAND | MF_ENABLED,
            );
            let _ = EnableMenuItem(
                GetSystemMenu(self.hwnd, false),
                super::publication::PUBLISH_OBSERVE as u32,
                MF_BYCOMMAND | MF_ENABLED,
            );
            wheel::sync(self.hwnd, &self.state);
        }
        Ok(())
    }
    pub(crate) fn request_default_input(&self) {
        {
            let mut input = self.state.input.borrow_mut();
            if input.allowed && !input.remote {
                input.desired = true;
                input.pending = true;
                input.request = Some(true);
            }
        }
        unsafe {
            wheel::sync(self.hwnd, &self.state);
        }
    }
    pub(crate) fn take_input_request(&self) -> Option<bool> {
        self.state.input.borrow_mut().request.take()
    }
    pub(crate) fn defer_input_resume(&self) {
        self.state.input.borrow_mut().request = Some(true);
    }
    pub(crate) fn source_activation_requested(&self) -> bool {
        let input = self.state.input.borrow();
        input.allowed
            && input.desired
            && !input.remote
            && !input.failed
            && (input.request == Some(true)
                || input.events.iter().any(|event| *event != Event::Leave))
    }
    pub(crate) fn input_resume_pending(&self) -> bool {
        self.state.input.borrow().request == Some(true)
    }
    pub(crate) fn pop_input(&self) -> Option<Event> {
        self.state.input.borrow_mut().events.pop_front()
    }
    pub(in crate::desktop) fn pop_dispatch_input(
        &self,
        is_browser: impl FnOnce() -> bool,
    ) -> Option<Event> {
        queue::pop_for_dispatch(&mut self.state.input.borrow_mut().events, is_browser)
    }
    pub(crate) fn input_status(&self, enabled: bool, pending: bool, stop: Option<StopSignal>) {
        {
            let mut input = self.state.input.borrow_mut();
            if (enabled || pending) && input.failed {
                input.failed = false;
                input.notice = false;
                input.shown = None;
            }
            if !enabled && !pending {
                input.clear();
            } else {
                input.native_active(true);
            }
            input.enabled = enabled;
            input.pending = pending;
            input.stop = stop;
        }
        if self.is_alive() {
            unsafe {
                display_status(self.hwnd, &self.state, enabled, pending);
                wheel::sync(self.hwnd, &self.state);
            }
        }
    }
    pub(crate) fn input_failure(&self) {
        let changed = {
            let mut input = self.state.input.borrow_mut();
            let changed = !input.failed;
            input.cancel();
            input.failed = true;
            input.notice |= changed;
            changed
        };
        if changed && self.is_alive() {
            unsafe {
                menu_status(self.hwnd, &self.state, false, false);
                wheel::sync(self.hwnd, &self.state);
                let _ = SetWindowTextW(
                    self.hwnd,
                    &windows::core::HSTRING::from(crate::ui_text::text(
                        "integration-live-control-unavailable",
                    )),
                );
            }
        }
    }
}
impl LiveWindow {
    pub(crate) fn suspend_source_input(&self) {
        unsafe {
            suspend(self.hwnd, &self.state);
        }
    }
    pub(crate) fn resume_browser_input(&self) {
        // Restore only still-valid control intent; no event replay or overriding closure, locking, or cancellation.
        unsafe { resume(&self.state) }
    }
}
unsafe fn display_status(hwnd: HWND, state: &State, enabled: bool, pending: bool) {
    let changed =
        state.input.borrow_mut().shown.replace((enabled, pending)) != Some((enabled, pending));
    if changed {
        menu_status(hwnd, state, enabled, pending);
        let _ = CheckMenuItem(
            GetSystemMenu(hwnd, false),
            TOGGLE as u32,
            (MF_BYCOMMAND | if enabled { MF_CHECKED } else { MF_UNCHECKED }).0,
        );
        let title = windows::core::HSTRING::from(if enabled {
            crate::ui_text::text("integration-live-controlling")
        } else if pending {
            crate::ui_text::text("integration-live-enabling")
        } else if state.input.borrow().failed {
            crate::ui_text::text("integration-live-control-unavailable")
        } else {
            "Otter Live"
        });
        let _ = SetWindowTextW(hwnd, &title);
    }
    if !enabled && !pending && state.drag.get().is_none() && GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    let menu = GetSystemMenu(hwnd, false);
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        TOGGLE,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-control-source")),
    )
}
fn viewport(hwnd: HWND, state: &State) -> Option<crate::live_viewport::Viewport> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) }.ok()?;
    // Host may be temporarily taken during GPU/Win32 calls; read the same committed record rather than treating reentrancy as invalidation.
    state
        .presented
        .get([
            (rect.right - rect.left).max(0) as u32,
            (rect.bottom - rect.top).max(0) as u32,
        ])
        .map(|(_, view)| view)
}
pub(super) unsafe fn cancel(hwnd: HWND, state: &State) {
    state.popup_focus.borrow_mut().clear();
    state.input.borrow_mut().cancel();
    display_status(hwnd, state, false, false);
    wheel::sync(hwnd, state);
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
/// Local movement/focus loss releases source keys first while preserving the control toggle; recovery still passes original-worker admission.
pub(super) unsafe fn suspend(hwnd: HWND, state: &State) {
    state.popup_focus.borrow_mut().clear();
    {
        let mut input = state.input.borrow_mut();
        input.pause();
    }
    display_status(hwnd, state, false, false);
}
fn resumable(state: &State) -> bool {
    let input = state.input.borrow();
    input.allowed
        && input.desired
        && !state.pin.selected.get()
        && !input.remote
        && !input.failed
        && !state.closed.get()
        && !state.group_locked.get()
        && !state.group_hidden.get()
}
unsafe fn resume(state: &State) {
    if resumable(state)
        && ![0x11, 0x12, 0x10]
            .iter()
            .any(|key| windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(*key) < 0)
    {
        let mut input = state.input.borrow_mut();
        if !input.enabled && !input.pending {
            input.pending = true;
            input.request = Some(true);
        }
    }
}
/// Some means consumed; preserve system window behavior without input control.
pub(super) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<Result<()>> {
    if !state.popup.get() && super::menu::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    // Popup nested message pumps must not create new source-control or local-geometry transactions; closure/lifecycle handling remains reachable.
    if state.menu_open.get()
        && (matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP)
            || (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&message)
            || (message == WM_SYSCOMMAND && wp.0 & 0xfff0 == TOGGLE))
    {
        return Some(Ok(()));
    }
    if !state.popup.get() && super::art::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    if !state.popup.get() && super::local::drag::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    if message == WM_LBUTTONDOWN && state.pin.selected.replace(false) {
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
    }
    if super::pin::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    if message == WM_SYSCOMMAND && wp.0 & 0xfff0 == TOGGLE {
        let toggle = {
            let input = state.input.borrow();
            (input.allowed && !input.remote).then_some(!input.desired)
        };
        if let Some(enable) = toggle {
            if !enable {
                cancel(hwnd, state);
            } else {
                state.pin.selected.set(false);
                {
                    let mut input = state.input.borrow_mut();
                    input.desired = true;
                    input.pending = true;
                    input.request = Some(true);
                }
                display_status(hwnd, state, false, true);
                wheel::sync(hwnd, state);
            }
        }
        return Some(Ok(()));
    }
    if matches!(message, WM_KILLFOCUS | WM_CANCELMODE | WM_ENTERMENULOOP)
        || (message == WM_ACTIVATE && wp.0 & 0xffff == WA_INACTIVE as usize)
        || (message == WM_SIZE && wp.0 == SIZE_MINIMIZED as usize)
    {
        if popup::focus_handoff(hwnd, state, message, wp, lp) {
            return None;
        }
        if state.input.borrow().enabled || state.input.borrow().pending {
            eprintln!("native-live-input-pause: reason=window-message message=0x{message:04x}");
        }
        suspend(hwnd, state);
        return None;
    }
    if message == WM_CAPTURECHANGED {
        let held = state.input.borrow().buttons != 0;
        if held && !popup::focus_handoff(hwnd, state, message, wp, lp) {
            cancel(hwnd, state);
        }
        return None;
    }
    if let Some(result) = (!state.popup.get())
        .then(|| super::local::handle(hwnd, state, message, wp, lp))
        .flatten()
    {
        return Some(result);
    }
    if message == WM_SETFOCUS && GetForegroundWindow() == hwnd {
        resume(state);
    }
    if ime::handle(hwnd, state, message, wp) {
        return None;
    }
    let mouse = (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&message)
        || matches!(message, WM_MOUSEHOVER | WM_MOUSELEAVE);
    if !state.input.borrow().enabled {
        // Background wheel input also reaches Live directly; recover only after a content hit, while keyboard input still awaits worker admission.
        if !mouse || (!state.input.borrow().pending && !resumable(state)) {
            return None;
        }
    }
    // Preserve local system menus and close shortcuts; immediately revoke source input on menu entry/closure.
    if message == WM_SYSKEYDOWN && (wp.0 == 0x20 || wp.0 == 0x73) {
        return None;
    }
    use crate::desktop::window_input_event as event;
    if matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP) && wp.0 == 0xe5 {
        return Some(Ok(())); // VK_PROCESSKEY belongs to B's IME and is not a source virtual key.
    }
    if message == WM_CHAR {
        if let Ok(unit) = u16::try_from(wp.0) {
            if unit != 0 && !state.input.borrow_mut().push(Event::Character { unit }) {
                suspend(hwnd, state);
            }
        }
        return Some(Ok(()));
    }
    if mouse {
        // TrackMouseEvent sends no window messages; IMM operations, however, cannot span RefCell borrows.
        if let Err(error) = state.input.borrow_mut().tracking.observe(hwnd, message) {
            return Some(Err(error));
        }
    }
    if message == WM_MOUSELEAVE {
        let held = state.input.borrow().buttons != 0;
        if held && state.popup_noactivate.get() {
            // Nonactivating menus must not steal source-menu capture; leaving still releases forwarded presses.
            suspend(hwnd, state);
            return Some(Ok(()));
        }
        if !held && !state.input.borrow_mut().push(Event::Leave) {
            suspend(hwnd, state);
        }
        return Some(Ok(()));
    }
    if !event::recognized(message) && message != WM_MOUSEHOVER {
        return None;
    }
    let Some(view) = viewport(hwnd, state) else {
        // Without trusted coordinates, invent/replay no input; pausing merely because the worker is enabled would revoke the menu provenance cycle.
        // UI-held mouse/keyboard input must follow original cancellation cleanup; handed-off up remains with the original FIFO/worker.
        let held = {
            let input = state.input.borrow();
            input.buttons != 0 || input.held_keys.iter().any(|word| *word != 0)
        };
        if held {
            eprintln!(
                "native-live-input-pause: reason=viewport-unavailable message=0x{message:04x}"
            );
            suspend(hwnd, state);
        }
        return Some(Ok(()));
    };
    let held = state.input.borrow().buttons != 0;
    let decoded = event::decode(
        hwnd,
        if message == WM_MOUSEHOVER {
            WM_MOUSEMOVE
        } else {
            message
        },
        wp,
        lp,
        view,
        held,
    );
    let event = match decoded {
        Ok(Some(event)) => event,
        Ok(None) => {
            if mouse && !held && !state.input.borrow_mut().push(Event::Leave) {
                suspend(hwnd, state);
            }
            return Some(Ok(()));
        }
        Err(()) => {
            cancel(hwnd, state);
            return Some(Ok(()));
        }
    };
    let event = if message == WM_MOUSEHOVER {
        let Event::Move { point } = event else {
            return Some(Ok(()));
        };
        Event::Hover { point }
    } else {
        event
    };
    if message == WM_LBUTTONDOWN {
        if let Event::Button { point, .. } = event {
            state.input.borrow_mut().ime.click(point);
        }
    }
    #[cfg(test)]
    if matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
        eprintln!("native-live-wheel-decode: {event:?}");
    }
    let edge = event::button(event);
    if mouse {
        // As in the baseline, clicks focus only owned Live while the source stays in background. Focus calls may reenter synchronously.
        if !state.popup_noactivate.get() && edge.is_some_and(|(_, down)| down) && resumable(state) {
            let _ = SetForegroundWindow(hwnd);
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(hwnd));
        }
        resume(state);
        let input = state.input.borrow();
        if state.closed.get()
            || state.group_locked.get()
            || state.group_hidden.get()
            || input.remote
            || input.failed
            || (!input.enabled && !input.pending)
        {
            return Some(Ok(()));
        }
    }
    if let Some((mask, down)) = edge {
        if !state.input.borrow_mut().button_edge(mask, down) {
            return Some(Ok(()));
        }
    }
    let accepted = state.input.borrow_mut().push(event);
    if !accepted {
        suspend(hwnd, state);
    } else if !state.popup_noactivate.get() && edge.is_some_and(|(_, down)| down) {
        let _ = SetCapture(hwnd);
        if GetCapture() != hwnd {
            cancel(hwnd, state);
        }
    } else if edge.is_some_and(|(_, down)| !down) {
        let held = state.input.borrow().buttons != 0;
        if !held && GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
    }
    Some(Ok(()))
}

#[cfg(test)]
#[path = "composition_live_input_overload_tests.rs"]
mod overload_tests;
