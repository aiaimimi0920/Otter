//! Software HWND style and first-frame submission form one transaction; hold only immutable DIBs across reentrancy, never borrow Canvas.
use super::super::{canvas::Pending, software_surface::window_style::Transition, State};
use windows::{core::Result, Win32::Foundation::HWND};

pub(super) unsafe fn submit(hwnd: HWND, state: &State, pending: Pending) -> Result<bool> {
    let owns = || state.lifetime.owns(hwnd);
    if !owns() {
        return Ok(false);
    }
    let transition = Transition::begin_guarded(hwnd, pending.pinned, &owns)?;
    if !state.canvas.borrow().accepts_pending(&pending) {
        state.render_deferred.set(true);
        return Ok(false); // Roll back style without submitting the old document frame from before reentrancy.
    }
    let position = state
        .pin_resize
        .rect
        .get()
        .map(|rect| windows::Win32::Foundation::POINT {
            x: rect.x,
            y: rect.y,
        });
    let presented = pending.present_at(hwnd, position)?;
    if !owns() {
        return Ok(false);
    }
    if !state.canvas.borrow().accepts_presented(&presented) {
        state.render_deferred.set(true);
        return Ok(false);
    }
    transition.commit()?;
    Ok(state.canvas.borrow_mut().confirm_submission(presented))
}
