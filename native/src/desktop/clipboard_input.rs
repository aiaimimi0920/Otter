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

// 附属按钮仍属于钉图；文本输入保留文字复制，不提升为全局快捷键。
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
