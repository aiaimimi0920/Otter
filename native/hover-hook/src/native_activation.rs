//! 仅补普通点击的源队列活动窗口；不改变前台、系统鼠标或已有模态活动窗。
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
        // Request 已限定本源线程。后台队列的 SetActiveWindow 不等于
        // SetForegroundWindow；菜单由应用自己的 owner 链确认，不延长 SHOW 租约。
        // 返回值是旧活动 HWND，旧值为 NULL 不表示本次失败，故以当前状态确认。
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
