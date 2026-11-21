//! 固定 Acquire/Restore 的源线程入口；不接受任意消息、指针、全局钩子或重放输入。
use crate::native_layer_protocol::{self as protocol, Operation, Request};
use std::sync::OnceLock;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub fn message_id() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(protocol::MESSAGE_NAME) })
}
/// true 仅表示已领取且发布终态；是否成功由原请求的 RESULT 决定。
pub fn dispatch(window: HWND, nonce: usize, receiving: HWND) -> windows::core::Result<bool> {
    let Some(request) = Request::read(window, nonce, receiving) else {
        return Ok(false);
    };
    if !request.claim()? {
        return Ok(false);
    }
    let result = match request.operation {
        Operation::Acquire => {
            crate::native_popup_layer::acquire_managed(request.lease, || request.active())
        }
        // 恢复不依赖 B 存活/控制 epoch；仍严格受原显示实例和 MANAGED 限制。
        Operation::Restore => crate::native_popup_layer::restore_managed(request.lease),
    };
    request.finish(result.err().map_or(S_OK, |error| error.code()))?;
    Ok(true)
}

/// # Safety
/// Windows 非负 WH_CALLWNDPROC 的 lp 为有效 CWPSTRUCT，始终继续原 hook 链。
/// 独立恢复 owner 还须安装 GETMESSAGE/原 RETURN hook，输入 worker 卸载后仍保留清理。
#[no_mangle]
pub unsafe extern "system" fn otter_live_layer_call(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && lp.0 != 0 {
        let call = &*(lp.0 as *const CWPSTRUCT);
        crate::native_popup::before(call);
        let id = message_id();
        if id != 0 && call.message == id {
            if let Err(error) = dispatch(
                HWND(call.wParam.0 as *mut _),
                call.lParam.0 as usize,
                call.hwnd,
            ) {
                // 无 FINISHED 时是未知结果，宿主不能退休或重发；保留实际发布错误。
                eprintln!("native-live-popup-layer-dispatch: {error}");
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}

/// # Safety
/// 非负 WH_GETMESSAGE 的 lp 为 Windows 保活的 MSG；只在 PM_REMOVE 消费固定标量请求。
/// 宿主可始终 PostMessage，避免同线程 SendNotifyMessage 退化为同步重入 UI。
#[no_mangle]
pub unsafe extern "system" fn otter_live_layer_get(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && wp.0 == PM_REMOVE.0 as usize && lp.0 != 0 {
        let message = &mut *(lp.0 as *mut MSG);
        let id = message_id();
        if id != 0 && message.message == id {
            match dispatch(
                HWND(message.wParam.0 as *mut _),
                message.lParam.0 as usize,
                message.hwnd,
            ) {
                Ok(true) => message.message = WM_NULL,
                Ok(false) => {}
                Err(error) => eprintln!("native-live-popup-layer-dispatch: {error}"),
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
