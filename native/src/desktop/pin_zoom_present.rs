//! Wheel presentation transaction: GPU prepares the new frame before client geometry commits; CPU commits pixels and the target rectangle atomically.
use super::{PinRect, State};
use std::cell::Cell;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

#[derive(Default)]
pub(in crate::desktop) struct Resize {
    pub rect: Cell<Option<PinRect>>,
    // This means the original paint chain completed without error, not confirmed_paints or DWM display; occlusion retains existing repaint semantics.
    pub prepared: Cell<bool>,
}

struct Scope<'a>(&'a Resize);
impl Drop for Scope<'_> {
    fn drop(&mut self) {
        self.0.rect.set(None);
        self.0.prepared.set(false);
    }
}

pub(in crate::desktop) unsafe fn apply(hwnd: HWND, state: &State, after: PinRect) -> bool {
    // Do not replace pending geometry during reentrant painting or another resize.
    if state.render_active.get() || state.pin_resize.rect.get().is_some() {
        return false;
    }
    state.pin_resize.rect.set(Some(after));
    state.pin_resize.prepared.set(false);
    let scope = Scope(&state.pin_resize);
    // The region also clips visible pixels; moving the old region with the new origin makes the right edge jump backward.
    // Remove old clipping on the synchronous stack, then restore the exact alpha region before returning to the input pump.
    if !super::preview::active(state) && SetWindowRgn(hwnd, None, false) == 0 {
        *state.error.borrow_mut() = Some(windows::core::Error::from_win32());
        return false;
    }
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    super::fit_size(hwnd, state, (after.width as i32, after.height as i32));
    crate::desktop::application::chrome::zoom_frame(&state.chrome, hwnd, after);
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    let software = state.canvas.borrow().software_selected();
    let positioned = if software && !super::preview::active(state) {
        // Do not move the old bitmap with SetWindowPos first; ULW_ALPHA uses this transaction's target position and new pixels.
        crate::desktop::procedure::resize_frame(hwnd, state);
        state.pin_resize.prepared.get()
    } else {
        // NCCALCSIZE arrives before window geometry takes effect; do not wait for WM_SIZE to ResizeBuffers/Present.
        SetWindowPos(
            hwnd,
            None,
            after.x,
            after.y,
            after.width as i32,
            after.height as i32,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS | SWP_NOREDRAW,
        )
        .is_ok()
    };
    let prepared = state.pin_resize.prepared.get();
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    super::fit(hwnd, state);
    // On success or failure, restore the actual client's exact alpha region; do not leave rectangular hit bounds.
    // Keep the resize scope until region restoration finishes, so caching cannot skip undoing the temporary unclipped state.
    let region = crate::desktop::hit_region::update(hwnd, state);
    drop(scope);
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    crate::desktop::application::chrome::geometry(&state.chrome, hwnd, WM_WINDOWPOSCHANGED);
    if let Err(error) = region {
        *state.error.borrow_mut() = Some(error);
        return false;
    }
    if positioned && prepared {
        // The same new-size frame already covers WM_SIZE's invalid region at geometry commit; do not draw another frame.
        let _ = ValidateRect(Some(hwnd), None);
    } else {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let _ = UpdateWindow(hwnd);
    }
    // Zoom accumulation follows actual HWND geometry. Fine steps may only change rounded position without sending NCCALCSIZE;
    // even if ordinary supplementary painting runs, do not zero subpixel accumulation for a successful reposition.
    positioned
}

pub(in crate::desktop) unsafe fn before_geometry(
    hwnd: HWND,
    state: &State,
    wp: WPARAM,
    lp: LPARAM,
) {
    if super::preview::active(state) {
        return;
    }
    if wp.0 == 0 || lp.0 == 0 || state.pin_resize.prepared.get() || state.render_active.get() {
        return;
    }
    let Some(rect) = state.pin_resize.rect.get() else {
        return;
    };
    let client = (*(lp.0 as *const NCCALCSIZE_PARAMS)).rgrc[0];
    // Handle only the expected client area of this ordinary borderless pin; other layout/DPI messages follow the original path.
    if client.right - client.left == rect.width as i32
        && client.bottom - client.top == rect.height as i32
        && !state.canvas.borrow().software_selected()
    {
        crate::desktop::procedure::resize_frame(hwnd, state);
    }
}
