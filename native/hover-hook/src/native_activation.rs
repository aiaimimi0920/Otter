//! Restore only source-queue active windows for ordinary clicks; never change foreground, system mouse, or existing modal active windows.
use crate::native_protocol::Request;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

fn activating_button(message: u32) -> bool {
    matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_RBUTTONUP)
}

pub(crate) fn prepare(request: Request) -> windows::core::Result<()> {
    unsafe {
        if !activating_button(request.message)
            || GetForegroundWindow() != request.owner
            || !GetActiveWindow().0.is_null()
            || GetWindowLongPtrW(request.root, GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE.0 != 0
            || !IsWindowVisible(request.root).as_bool()
            || !IsWindowEnabled(request.root).as_bool()
        {
            return Ok(());
        }
        // Request is already restricted to this source thread. SetActiveWindow on a background queue is not
        // SetForegroundWindow; the application's own owner chain confirms menus without extending SHOW leases.
        // The return value is the previous active HWND; NULL does not indicate failure, so confirm through current state.
        let _ = SetActiveWindow(request.root);
        if GetForegroundWindow() != request.owner || GetActiveWindow() != request.root {
            return Err(windows::core::Error::new(
                E_ABORT,
                "Live source activation changed",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_click_activation_edges_prepare_the_source_queue() {
        for message in [WM_LBUTTONDOWN, WM_LBUTTONDBLCLK, WM_RBUTTONUP] {
            assert!(activating_button(message));
        }
        for message in [
            WM_LBUTTONUP,
            WM_RBUTTONDOWN,
            WM_MOUSEMOVE,
            WM_MOUSEWHEEL,
            WM_KEYDOWN,
            WM_COMMAND,
            windows::Win32::UI::Controls::WM_MOUSEHOVER,
        ] {
            assert!(!activating_button(message));
        }
    }
}
