//! 画布剪贴板快捷键和复制定时器路由；不在消息总入口累积编解码或生命周期逻辑。
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
) -> Option<LRESULT> {
    match message {
        WM_TIMER if wp.0 == super::copy_job::TIMER => super::copy_job::poll(hwnd, state),
        WM_KEYDOWN if wp.0 == 0x43 && GetKeyState(VK_CONTROL.0 as i32) < 0 => {
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
