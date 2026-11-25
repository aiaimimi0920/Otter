//! Prepare wheel frames before HWND geometry commits; afterward retain default size notifications for viewport/shared-chrome adaptation.
use super::State;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

// System dragging uses only the attached windows' existing geometry-following path. Preserve default message dispatch and queued wakes,
// but they must not repeatedly recalculate tool/settings fields; size, DPI, task, and drag-end refreshes remain unchanged.
pub(super) unsafe fn drag_position_only(state: &State, msg: u32, lp: LPARAM) -> bool {
    if !crate::desktop::pin_window::animation::moving(state) {
        return false;
    }
    match msg {
        WM_MOVING | WM_MOVE => true,
        WM_WINDOWPOSCHANGED if lp.0 != 0 => {
            let flags = (*(lp.0 as *const WINDOWPOS)).flags;
            flags.contains(SWP_NOSIZE)
                && (flags & (SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_HIDEWINDOW)).0 == 0
        }
        _ => false,
    }
}

pub(super) unsafe fn before_dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    crate::desktop::pin_window::preview::click_dispatch(hwnd, state, msg, wp, lp)
        .or_else(|| crate::desktop::pin_window::animation::dispatch(hwnd, state, msg, wp))
        .or_else(|| crate::desktop::session_end::dispatch(hwnd, state, msg, wp, lp))
}

pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !matches!(
        msg,
        WM_NCCALCSIZE | WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED
    ) {
        return None;
    }
    let result = DefWindowProcW(hwnd, msg, wp, lp);
    if state.lifetime.owns(hwnd) {
        if msg == WM_WINDOWPOSCHANGING && lp.0 != 0 && state.pin_resize.rect.get().is_none() {
            let pos = &*(lp.0 as *const WINDOWPOS);
            let mut old = RECT::default();
            if !pos.flags.contains(SWP_NOMOVE | SWP_NOSIZE) && GetWindowRect(hwnd, &mut old).is_ok()
            {
                let (x, y) = if pos.flags.contains(SWP_NOMOVE) {
                    (old.left, old.top)
                } else {
                    (pos.x, pos.y)
                };
                // Ordinary movement also prepares the target border before host position takes effect, rather than chasing the moved old image with a white border.
                let size = if pos.flags.contains(SWP_NOSIZE) {
                    (old.right - old.left, old.bottom - old.top)
                } else {
                    (pos.cx, pos.cy)
                };
                if size.0 > 0 && size.1 > 0 {
                    crate::desktop::pin_window::movement::position(
                        hwnd,
                        state,
                        crate::pin_geometry::PinRect {
                            x,
                            y,
                            width: size.0 as u32,
                            height: size.1 as u32,
                        },
                    );
                    crate::desktop::application::chrome::zoom_frame(
                        &state.chrome,
                        hwnd,
                        crate::pin_geometry::PinRect {
                            x,
                            y,
                            width: size.0 as u32,
                            height: size.1 as u32,
                        },
                    );
                }
            }
        } else if msg == WM_NCCALCSIZE {
            crate::desktop::pin_window::presentation::before_geometry(hwnd, state, wp, lp);
        } else if msg == WM_WINDOWPOSCHANGED && state.pin_resize.rect.get().is_none() {
            // The default procedure first dispatches WM_SIZE and adapts the viewport; only then may transparent overlays synchronously trigger host painting.
            crate::desktop::application::chrome::geometry(&state.chrome, hwnd, msg);
            crate::desktop::tool_panel::follow(hwnd, state, msg);
            crate::desktop::device_authorization_ui::share::follow(hwnd, state);
        }
    }
    Some(result)
}
