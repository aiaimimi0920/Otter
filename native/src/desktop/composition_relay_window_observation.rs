//! Explicit read-only viewing entry; reuse original Host snapshots/notices, closing only the inspection window on failure without disturbing viewing/documents.
use super::*;
#[path = "composition_relay_observation_window.rs"]
mod window;
pub(super) use window::Window;
pub(crate) const OPEN: usize = 0x1fe0;
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    AppendMenuW(
        GetSystemMenu(hwnd, false),
        MF_STRING,
        OPEN,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-view-observations")),
    )
}
fn snapshot(state: &State) -> Option<Arc<crate::live_relay_viewer_business::Business>> {
    state
        .host
        .borrow()
        .as_ref()
        .and_then(Host::business_snapshot)
}
unsafe fn label(hwnd: HWND, state: &State, failed: bool) {
    if state.closed.get()
        || GetWindowLongPtrW(hwnd, GWLP_USERDATA) != state as *const State as isize
    {
        return;
    }
    let language = crate::ui_text::current_language();
    state.observation_label.set((None, failed));
    if ModifyMenuW(
        GetSystemMenu(hwnd, false),
        OPEN as u32,
        MF_BYCOMMAND | MF_STRING,
        OPEN,
        &windows::core::HSTRING::from(crate::ui_text::text_for(
            language,
            if failed {
                "integration-view-observations-failed"
            } else {
                "integration-view-observations"
            },
        )),
    )
    .is_ok()
    {
        state.observation_label.set((Some(language), failed));
    }
}
pub(super) unsafe fn refresh(hwnd: HWND, state: &State) {
    let (language, failed) = state.observation_label.get();
    if !state.closed.get() && language != Some(crate::ui_text::current_language()) {
        label(hwnd, state, failed);
    }
    let window = state.observation.borrow_mut().take();
    let Some(mut window) = window else {
        return;
    };
    if !state.closed.get() && window.handle().is_some() {
        if window.update(snapshot(state)).is_err() {
            label(hwnd, state, true);
            return;
        }
        if !state.closed.get() && window.handle().is_some() {
            *state.observation.borrow_mut() = Some(window);
        }
    }
}
pub(super) unsafe fn handle(hwnd: HWND, state: &State, message: u32, wp: WPARAM) -> bool {
    if message != WM_SYSCOMMAND || wp.0 & 0xfff0 != OPEN {
        return false;
    }
    if state.closed.get() {
        return true;
    }
    // Even when opened through owned messages outside the system menu, release remote keys/buttons first to avoid accidentally controlling Source during local inspection.
    input::release(hwnd, state);
    let previous = state.observation.borrow_mut().take();
    let window = previous
        .filter(|w| w.handle().is_some())
        .map(Ok)
        .unwrap_or_else(|| Window::new(hwnd));
    match window {
        Ok(mut window) => {
            if window.update(snapshot(state)).is_err() {
                label(hwnd, state, true);
                return true;
            }
            if !state.closed.get()
                && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == state as *const State as isize
            {
                window.show();
                if !state.closed.get() {
                    *state.observation.borrow_mut() = Some(window);
                }
            }
        }
        Err(_) => label(hwnd, state, true),
    }
    true
}
#[cfg(test)]
impl RelayWindow {
    pub(crate) fn observation_test(&self) -> Option<(HWND, HWND)> {
        self.state
            .observation
            .borrow()
            .as_ref()
            .and_then(|w| w.handle().map(|hwnd| (hwnd, w.edit())))
    }
}
