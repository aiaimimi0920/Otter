//! Clean input when yielding; change Z-order after the model borrow ends to prevent synchronous-message reentrancy.
use super::{window_priority, State};
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::InvalidateRect,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

pub(super) unsafe fn refresh(hwnd: HWND, state: &State, claim: bool) -> Result<()> {
    if !state.canvas.borrow().pinned {
        return Ok(());
    }
    let foreground = GetForegroundWindow();
    let blocker = window_priority::blocker(hwnd);
    let anchor = state
        .priority_policy
        .borrow_mut()
        .resolve(
            foreground.0 as isize,
            blocker.map(|window| window.0 as isize),
            claim,
        )
        .map(|handle| HWND(handle as *mut _));
    let key = anchor.map_or(0, |window| window.0 as isize);
    if state.priority_anchor.get() == key {
        return Ok(());
    }
    if anchor.is_some() {
        let changed = state.canvas.borrow_mut().cancel_gesture();
        if GetCapture() == hwnd {
            ReleaseCapture()?;
        }
        if changed {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
    // Reorder only when the actual anchor changes; background refreshes must not continually reclaim topmost.
    SetWindowPos(
        hwnd,
        Some(anchor.unwrap_or(HWND_TOPMOST)),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    )?;
    state.priority_anchor.set(key);
    Ok(())
}
