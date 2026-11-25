//! Retain the same complete presented frame after input stops; idle does not switch visible layers or block workspace snapshots.
use super::{PinRect, State};
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(in crate::desktop) fn active(state: &State) -> bool {
    state.pin_animation.parked.get()
}
pub(in crate::desktop) fn eligible(state: &State) -> bool {
    let canvas = state.canvas.borrow();
    canvas.pin_move_ready() && canvas.minified.is_none() && super::preview::shared(state)
}
pub(super) fn park(state: &State) -> bool {
    if !eligible(state) || !super::preview::fresh(state) {
        return false;
    }
    state.pin_animation.parked.set(true);
    true
}
pub(in crate::desktop) unsafe fn refresh(hwnd: HWND, state: &State) -> Result<()> {
    if !active(state) || state.pin_animation.refreshing.replace(true) {
        return Ok(());
    }
    struct Refresh<'a>(&'a State);
    impl Drop for Refresh<'_> {
        fn drop(&mut self) {
            self.0.pin_animation.refreshing.set(false);
        }
    }
    let _scope = Refresh(state);
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect)?;
    super::preview::refresh_snapshot(
        hwnd,
        state,
        PinRect {
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left).max(1) as u32,
            height: (rect.bottom - rect.top).max(1) as u32,
        },
    )
}
