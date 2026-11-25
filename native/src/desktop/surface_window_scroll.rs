//! Local wheel routing reads system preferences and converts physical screen coordinates to client coordinates, without Surface actions.
use super::*;
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if !state.local_edit.get() || state.closed.get() {
        return false;
    }
    let Some(input) = crate::desktop::surface_scroll_input::wheel(hwnd, message, wp, lp) else {
        return false;
    };
    let mut changed = false;
    let result = state.run(|host| {
        changed = host.scroll(input.pixel, input.axis, input.delta, input.page)?;
        Ok(())
    });
    if let Err(error) = result {
        // EDIT repositioning may fail after presentation commits; failure also discards input candidates on old geometry.
        pointer::invalidate(hwnd, state);
        let _ = InvalidateRect(Some(hwnd), None, false);
        *state.error.borrow_mut() = Some(error);
        return true;
    }
    if changed {
        // After geometry changes, old presses/pending clicks cannot re-hit nodes at new positions.
        pointer::invalidate(hwnd, state);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    changed
}
