//! HWND-private recovery timer: poll only the existing retired owner; before completion, do not paint, rebuild, or queue window closure.
use super::super::State;
use crate::desktop::composition_retirement::Status;
use windows::Win32::{Foundation::HWND, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*};

pub(super) const TIMER: usize = 0x4F63;

pub(in crate::desktop) unsafe fn start(hwnd: HWND, state: &State) {
    if !state.lifetime.owns(hwnd) {
        return;
    }
    // No Canvas borrow crosses Win32 scheduling calls; failure preserves the original paint error without starting another candidate.
    if SetTimer(Some(hwnd), TIMER, 16, None) == 0 {
        state.canvas.borrow_mut().suspend_render();
        eprintln!("native-render-recovery: timer=false document=retained");
        return;
    }
    poll(hwnd, state);
}

pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    if !state.lifetime.owns(hwnd) {
        return;
    }
    // If reentering active paint, defer until the next tick; no borrow conflicts or owner replacement during the outer frame.
    if state.render_active.get() {
        return;
    }
    let status = state.canvas.borrow_mut().poll_render_recovery();
    if status == Some(Status::Pending) {
        return;
    }
    let _ = KillTimer(Some(hwnd), TIMER);
    match status {
        Some(Status::Complete) => {
            eprintln!("native-render-recovery: retirement=complete next-candidate=admitted document=retained");
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        Some(status) => {
            eprintln!("native-render-recovery: retirement={status:?} document=retained")
        }
        None => {} // Queued stale WM_TIMER messages neither recreate, invalidate, nor clear the original error.
    }
}

pub(super) unsafe fn close(hwnd: HWND) {
    let _ = KillTimer(Some(hwnd), TIMER);
}
