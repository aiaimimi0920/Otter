//! Canvas clipboard shortcuts and copy-timer routing; do not accumulate codecs or lifecycle logic in the main message entry point.
use super::State;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::InvalidateRect,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    match message {
        WM_TIMER if wp.0 == super::copy_job::TIMER => super::copy_job::poll(hwnd, state),
        WM_KEYDOWN if wp.0 == 0x43 && GetKeyState(VK_CONTROL.0 as i32) < 0 => {
            if lp.0 & (1 << 30) != 0 {
                return Some(LRESULT(0));
            }
            if GetCapture() == hwnd {
                let _ = ReleaseCapture();
            }
            super::copy_job::start(hwnd, state);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_KEYDOWN if wp.0 == 0x56 && GetKeyState(VK_CONTROL.0 as i32) < 0 => {
            super::paste_input::start(hwnd, state)
        }
        _ => return None,
    }
    Some(LRESULT(0))
}

// Attached buttons still belong to the pin; text input retains text copy and does not become a global shortcut.
pub(super) unsafe fn translate(owner: HWND, message: &MSG) -> bool {
    if message.message != WM_KEYDOWN
        || message.wParam.0 != 0x43
        || GetKeyState(VK_CONTROL.0 as i32) >= 0
    {
        return false;
    }
    let mut class = [0u16; 32];
    let length = GetClassNameW(message.hwnd, &mut class) as usize;
    if String::from_utf16_lossy(&class[..length])
        .to_ascii_lowercase()
        .contains("edit")
    {
        return false;
    }
    SendMessageW(
        owner,
        message.message,
        Some(message.wParam),
        Some(message.lParam),
    );
    true
}
