//! 应用级 Ctrl+1 开关：请求合并，状态共享，系统注册仍由唯一宿主拥有。
use super::{document_window::DocumentWindow, State};
#[path = "application_hotkey_smoke.rs"]
mod smoke;
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
        // 读取真实注册标志，而不是菜单缓存；另一个窗口也能关闭当前唯一注册者。
        let owner = documents
            .iter()
            .find(|document| document.is_alive() && document.state.global_capture.get());
        let target = owner.unwrap_or(source);
        if let Err(error) = change(target.window.0, &target.state, owner.is_none()) {
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
