//! Expansion transaction for restored minimized windows; physical coordinates come from the same restore conversion, not guessed historical DPR.
use super::*;
use windows::core::{Error, Result};
use windows::Win32::Graphics::Gdi::SetWindowRgn;

pub(in crate::desktop) unsafe fn restore(hwnd: HWND, state: &State) -> Result<bool> {
    if !super::toggle_guard::allows(state) {
        return Ok(false);
    }
    let transition = {
        let canvas = state.canvas.borrow();
        let Some(presentation) = canvas.minified.as_ref() else {
            return Ok(false);
        };
        if !presentation.matches(&canvas.editor) {
            return Ok(false);
        }
        canvas.minified_restore
    };
    let Some(transition) = transition else {
        return Ok(false);
    };
    super::minify::prepare(hwnd, state)?;
    let opacity = transition.opacity;
    let mut current = RECT::default();
    GetWindowRect(hwnd, &mut current)?;
    let target = transition
        .target((current.left, current.top))
        .ok_or_else(|| Error::new(E_INVALIDARG, "Expanded window coordinates overflow"))?;
    let (presentation, previous_opacity, viewport) = {
        let mut canvas = state.canvas.borrow_mut();
        let opacity_before = canvas.window_opacity;
        canvas.window_opacity = opacity;
        (canvas.minified.take(), opacity_before, canvas.viewport)
    };
    if let Err(error) = super::minify::present(hwnd, state, target) {
        let mut canvas = state.canvas.borrow_mut();
        canvas.minified = presentation;
        canvas.window_opacity = previous_opacity;
        canvas.viewport = viewport;
        drop(canvas);
        super::preview::finish(hwnd, state, true);
        return Err(error);
    }
    state.canvas.borrow_mut().minified_restore = None;
    fit(hwnd, state);
    let no_image = state.canvas.borrow().image.is_none();
    if no_image && !super::preview::active(state) && SetWindowRgn(hwnd, None, false) == 0 {
        return Err(Error::from_win32());
    }
    super::super::hit_region::update(hwnd, state)?;
    let _ = InvalidateRect(Some(hwnd), None, false);
    Ok(true)
}
