//! Application-wide Ctrl+1 toggle: coalesce requests and share state, while a single host still owns system registration.
use super::{document_window::DocumentWindow, State};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_hotkey_smoke.rs"]
mod smoke;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) use smoke::run as smoke;
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    if state.file_dialog.get() || state.hotkey_toggle.replace(true) {
        return Ok(());
    }
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        state.hotkey_toggle.set(false);
        return Err(error);
    }
    Ok(())
}

pub(super) unsafe fn process_with(
    documents: &[DocumentWindow],
    mut change: impl FnMut(HWND, &State, bool) -> Result<()>,
    mut failed: impl FnMut(HWND, windows::core::Error),
) {
    for source in documents {
        if !source.state.hotkey_toggle.replace(false) || !source.is_alive() {
            continue;
        }
        // Read the actual registration flag, not the menu cache; another window can also disable the sole current registrant.
        let owner = documents.iter().find(|document| {
            document.is_alive() && super::shortcut_registry::owns(&document.state)
        });
        let target = owner.unwrap_or(source);
        if let Err(error) = change(
            target.window.0,
            &target.state,
            !target.state.hotkey_enabled.get(),
        ) {
            failed(source.window.0, error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modal_request_is_ignored_and_failed_wakeup_is_retryable() {
        let state = State::new(None, false).unwrap();
        state.file_dialog.set(true);
        unsafe {
            request(HWND::default(), &state).unwrap();
        }
        assert!(!state.hotkey_toggle.get());
        state.file_dialog.set(false);
        for _ in 0..2 {
            assert!(unsafe { request(HWND(-2isize as *mut _), &state) }.is_err());
            assert!(!state.hotkey_toggle.get());
        }
    }
}
