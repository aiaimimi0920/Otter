//! Gesture cleanup/presentation synchronization at menu boundaries; command catalogs do not duplicate capture release or pin hit-region updates.
use super::State;
use windows::{
    core::Result,
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::InvalidateRect,
        UI::{
            Input::KeyboardAndMouse::{GetCapture, ReleaseCapture},
            WindowsAndMessaging::IsWindow,
        },
    },
};

pub(super) unsafe fn prepare(hwnd: HWND, state: &State) -> Result<()> {
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        ReleaseCapture()?;
    }
    Ok(())
}

pub(super) unsafe fn finish(hwnd: HWND, state: &State) -> Result<()> {
    if IsWindow(Some(hwnd)).as_bool() {
        super::tool_panel::invalidate(state);
        super::crop_window::sync(hwnd, state)?;
        super::hit_region::update(hwnd, state)?;
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Ok(())
}
