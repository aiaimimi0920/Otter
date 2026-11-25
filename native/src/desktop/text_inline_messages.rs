//! 原生 EDIT 保留剪贴板、选择和 IME 默认处理；只在组合结束后解释提交/取消。
use super::*;
use windows::Win32::UI::Shell::*;
pub(super) const FINISH: u32 = WM_APP + 0x583;
pub(super) const CHANGE: u32 = WM_APP + 0x584;
pub(super) const CANCEL: u32 = WM_APP + 0x585;
const ID: usize = 0x4f544954;
pub(super) unsafe fn install(edit: HWND) -> Result<()> {
    if SetWindowSubclass(edit, Some(key), ID, 0).as_bool() {
        Ok(())
    } else {
        Err(Error::from_win32())
    }
}
unsafe fn send(popup: HWND, message: u32, posted: bool) {
    let owner = HWND(GetWindowLongPtrW(popup, GWLP_USERDATA) as *mut _);
    if desktop::host_lifetime::owns(owner) {
        let epoch = WPARAM(GetPropW(popup, window::EPOCH).0 as usize);
        let sender = LPARAM(popup.0 as isize);
        if posted {
            if let Err(error) = PostMessageW(Some(owner), message, epoch, sender) {
                eprintln!("native-inline-text-post: {error}");
            }
        } else {
            SendMessageW(owner, message, Some(epoch), Some(sender));
        }
    }
}
unsafe extern "system" fn key(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    composing: usize,
) -> LRESULT {
    if desktop::session_end::blocks_input(msg) {
        return LRESULT(0);
    }
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(key), ID);
    }
    if msg == WM_IME_STARTCOMPOSITION {
        let _ = SetWindowSubclass(hwnd, Some(key), ID, 1);
    }
    if composing == 0
        && matches!(msg, WM_KEYDOWN | WM_CHAR | WM_KEYUP)
        && (wp.0 == VK_ESCAPE.0 as usize
            || wp.0 == VK_RETURN.0 as usize && GetKeyState(VK_SHIFT.0 as i32) >= 0)
    {
        if msg == WM_KEYDOWN && lp.0 as usize & (1 << 30) == 0 {
            if let Ok(parent) = GetParent(hwnd) {
                send(
                    parent,
                    if wp.0 == VK_RETURN.0 as usize {
                        FINISH
                    } else {
                        CANCEL
                    },
                    false,
                );
            }
        }
        return LRESULT(0);
    }
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if msg == WM_IME_ENDCOMPOSITION && IsWindow(Some(hwnd)).as_bool() {
        let _ = SetWindowSubclass(hwnd, Some(key), ID, 0);
    }
    if msg == WM_KILLFOCUS {
        if let Ok(parent) = GetParent(hwnd) {
            // WM_KILLFOCUS 中不关窗/激活其他窗口；随后核验实例身份再提交回收。
            send(parent, FINISH, true);
        }
    }
    result
}
pub(super) unsafe extern "system" fn popup(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    std::panic::catch_unwind(|| route(hwnd, msg, wp, lp)).unwrap_or_else(|_| {
        PostQuitMessage(1);
        LRESULT(0)
    })
}
unsafe fn route(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND if (wp.0 >> 16) as u16 == EN_CHANGE as u16 => {
            send(hwnd, CHANGE, false);
            LRESULT(0)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => paint::edit(hwnd, HDC(wp.0 as *mut _)),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => paint::popup(hwnd),
        WM_NCDESTROY => {
            paint::retire(hwnd);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_CLOSE => {
            send(hwnd, CANCEL, false);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
