//! 已绑定源线程的 Leave 过滤与私有按钮/悬浮/弹窗协议；无全局键鼠输入。
mod native_activation;
pub mod native_dispatch;
pub mod native_input_scope;
pub mod native_layer_dispatch;
pub mod native_layer_protocol;
mod native_nested;
pub mod native_popup;
pub mod native_popup_layer;
mod native_popup_placement;
pub mod native_protocol;
pub mod scope;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

/// # Safety
/// 仅由 Windows WH_GETMESSAGE 调用；非负 code 的 LPARAM 指向有效 MSG。
#[no_mangle]
pub unsafe extern "system" fn otter_live_hover_hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        native_dispatch::message_pump_entered();
    }
    if code >= 0 && wp.0 == PM_REMOVE.0 as usize && lp.0 != 0 {
        let message = &mut *(lp.0 as *mut MSG);
        if message.message == windows::Win32::UI::Controls::WM_MOUSELEAVE
            && scope::suppress(message.hwnd)
        {
            message.message = WM_NULL;
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
