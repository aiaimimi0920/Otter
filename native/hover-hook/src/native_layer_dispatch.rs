//! Source-thread entry for fixed Acquire/Restore; no arbitrary messages, pointers, global hooks, or input replay.
use crate::native_layer_protocol::{self as protocol, Operation, Request};
use std::sync::OnceLock;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub fn message_id() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(protocol::MESSAGE_NAME) })
}
/// true means only claimed and terminal state published; the original request's RESULT determines success.
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
        // Restoration does not depend on B survival/control epoch; original display-instance and MANAGED restrictions still apply.
        Operation::Restore => crate::native_popup_layer::restore_managed(request.lease),
    };
    request.finish(result.err().map_or(S_OK, |error| error.code()))?;
    Ok(true)
}

/// # Safety
/// For nonnegative Windows WH_CALLWNDPROC, lp is valid CWPSTRUCT; always continue the original hook chain.
/// Independent recovery owners must also install GETMESSAGE/original RETURN hooks, preserving cleanup after input-worker unload.
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
                // Without FINISHED, the result is unknown and hosts cannot retire/resend; preserve actual publication errors.
                eprintln!("native-live-popup-layer-dispatch: {error}");
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}

/// # Safety
/// Nonnegative WH_GETMESSAGE lp is a Windows-retained MSG; consume fixed scalar requests only with PM_REMOVE.
/// Hosts may always PostMessage, avoiding same-thread SendNotifyMessage degrading into synchronous UI reentry.
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
