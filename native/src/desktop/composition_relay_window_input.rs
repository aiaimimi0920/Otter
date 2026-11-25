//! Relay windows record only explicit control intent and bounded input; original control threads own networking, renewal, and release.
use super::*;
use crate::{
    desktop::window_input_event as event,
    live_relay_control_runtime::interaction::{Queue, Status},
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
pub(crate) const ACQUIRE: usize = 0x1fc0;
pub(crate) const RELEASE: usize = 0x1fd0;
#[derive(Default)]
pub(super) struct Control {
    pub(super) queue: Option<Arc<Queue>>,
    buttons: u8,
    shown: Option<(Status, crate::ui_text::DisplayLanguage)>,
    pub(super) trigger_shown: Option<(
        crate::live_relay_viewer_trigger_queue::Phase,
        bool,
        crate::ui_text::DisplayLanguage,
    )>,
}
impl Control {
    pub(super) fn stop(&mut self) {
        if let Some(queue) = &self.queue {
            queue.stop();
        }
        self.buttons = 0;
    }
}
impl RelayWindow {
    pub(crate) fn bind_input(&self, queue: Arc<Queue>) {
        self.state.input.borrow_mut().queue = Some(queue);
        unsafe {
            refresh(self.hwnd, &self.state);
            super::trigger::refresh(self.hwnd, &self.state);
        }
    }
    #[cfg(test)]
    pub(crate) fn input_test_hwnd(&self) -> HWND {
        self.hwnd
    }
    #[cfg(test)]
    pub(crate) fn input_test_status(&self) -> Status {
        queue(&self.state).map(|q| q.status()).unwrap_or_default()
    }
}
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    let menu = GetSystemMenu(hwnd, false);
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        ACQUIRE,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-acquire-remote-control")),
    )?;
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        RELEASE,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-release-remote-control")),
    )
}
fn queue(state: &State) -> Option<Arc<Queue>> {
    state.input.borrow().queue.clone()
}
pub(super) unsafe fn refresh(hwnd: HWND, state: &State) {
    if state.closed.get() {
        return;
    }
    let status = queue(state).map(|q| q.status()).unwrap_or_default();
    let language = crate::ui_text::current_language();
    let shown = (status, language);
    let changed = {
        let mut input = state.input.borrow_mut();
        if !status.enabled {
            input.buttons = 0;
        }
        input.shown != Some(shown)
    };
    if !status.enabled && GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    if !changed
        || state.closed.get()
        || GetWindowLongPtrW(hwnd, GWLP_USERDATA) != state as *const State as isize
    {
        return;
    }
    // A partial native write must remain retryable, including a switch back to the old language.
    state.input.borrow_mut().shown = None;
    let menu = GetSystemMenu(hwnd, false);
    let acquire = ModifyMenuW(
        menu,
        ACQUIRE as u32,
        MF_BYCOMMAND
            | MF_STRING
            | if status.available && !status.pending && !status.enabled {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        ACQUIRE,
        &windows::core::HSTRING::from(crate::ui_text::text_for(
            language,
            "integration-acquire-remote-control",
        )),
    );
    let release = ModifyMenuW(
        menu,
        RELEASE as u32,
        MF_BYCOMMAND
            | MF_STRING
            | if status.pending || status.enabled {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        RELEASE,
        &windows::core::HSTRING::from(crate::ui_text::text_for(
            language,
            "integration-release-remote-control",
        )),
    );
    let title = SetWindowTextW(
        hwnd,
        &windows::core::HSTRING::from(crate::ui_text::text_for(
            language,
            if status.recovering {
                "integration-remote-reconnecting"
            } else if status.enabled {
                "integration-remote-controlling"
            } else if status.pending {
                "integration-remote-acquiring"
            } else if status.failed {
                "integration-remote-control-failed"
            } else {
                "integration-remote-viewer"
            },
        )),
    );
    if acquire.is_ok() && release.is_ok() && title.is_ok() && !state.closed.get() {
        state.input.borrow_mut().shown = Some(shown);
    }
}
pub(super) unsafe fn release(hwnd: HWND, state: &State) {
    if let Some(queue) = queue(state) {
        queue.release();
    }
    state.input.borrow_mut().buttons = 0;
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    refresh(hwnd, state);
}
fn viewport(hwnd: HWND, state: &State) -> Option<crate::live_viewport::Viewport> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) }.ok()?;
    state.host.borrow().as_ref()?.presented_viewport([
        (rect.right - rect.left).max(0) as u32,
        (rect.bottom - rect.top).max(0) as u32,
    ])
}
pub(super) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if message == WM_SYSCOMMAND && matches!(wp.0 & 0xfff0, ACQUIRE | RELEASE) {
        if wp.0 & 0xfff0 == RELEASE {
            release(hwnd, state);
        } else if let Some(queue) = queue(state) {
            queue.request();
            refresh(hwnd, state);
        }
        return true;
    }
    if matches!(
        message,
        WM_KILLFOCUS | WM_CANCELMODE | WM_ENTERMENULOOP | WM_SIZE | WM_SHOWWINDOW
    ) || (message == WM_ACTIVATE && wp.0 & 0xffff == WA_INACTIVE as usize)
        || (message == WM_CAPTURECHANGED && state.input.borrow().buttons != 0)
    {
        release(hwnd, state);
        return false;
    }
    let Some(queue) = queue(state) else {
        return false;
    };
    if !queue.status().enabled || !event::recognized(message) {
        return false;
    }
    // Local Alt+Space/Alt+F4 remain system-owned; menu entry/closure revokes control.
    if message == WM_SYSKEYDOWN && (wp.0 == 0x20 || wp.0 == 0x73) {
        return false;
    }
    let Some(view) = viewport(hwnd, state) else {
        release(hwnd, state);
        return true;
    };
    let held = state.input.borrow().buttons != 0;
    let input = match event::decode(hwnd, message, wp, lp, view, held) {
        Ok(Some(e)) => e,
        Ok(None) => return true,
        Err(()) => {
            release(hwnd, state);
            return true;
        }
    };
    if !queue.push(input) {
        release(hwnd, state);
        return true;
    }
    if let Some((mask, down)) = event::button(input) {
        {
            let mut input = state.input.borrow_mut();
            if down {
                input.buttons |= mask;
            } else {
                input.buttons &= !mask;
            }
        }
        if down {
            let _ = SetCapture(hwnd);
            if GetCapture() != hwnd {
                release(hwnd, state);
            }
        } else if state.input.borrow().buttons == 0 && GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
    }
    true
}
