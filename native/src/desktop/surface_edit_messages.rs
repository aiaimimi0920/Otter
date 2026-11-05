//! EDIT 子类只转发组合/导航边沿；无 Rust 借用跨默认过程，无裸指针指向可释放会话。
#[path = "surface_number_paste.rs"]
pub(super) mod number_paste;
#[path = "surface_number_preflight.rs"]
mod number_preflight;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
};
pub const COMPOSITION_START: u32 = WM_APP + 0x551;
pub const COMPOSITION_END: u32 = WM_APP + 0x552;
pub const NAVIGATE: u32 = WM_APP + 0x553;
pub const COMMIT: u32 = WM_APP + 0x554;
pub const EDIT_BEGIN: u32 = WM_APP + 0x555;
pub const EDIT_END: u32 = WM_APP + 0x556;
pub const STEP_UP: u32 = WM_APP + 0x557;
pub const STEP_DOWN: u32 = WM_APP + 0x558;
pub const CAN_PASTE: u32 = WM_APP + 0x559;
pub const ENTER_KEY: u32 = WM_APP + 0x561;
const ID: usize = 0x4f545445;
pub fn install(hwnd: HWND, number: bool) -> Result<()> {
    if unsafe { SetWindowSubclass(hwnd, Some(procedure), ID, usize::from(number) << 1) }.as_bool() {
        Ok(())
    } else {
        Err(Error::from_win32())
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    composing: usize,
) -> LRESULT {
    if super::session_end::blocks_input(message) {
        return LRESULT(0);
    }
    if composing & 2 != 0
        && (matches!(
            message,
            WM_NCDESTROY
                | WM_IME_STARTCOMPOSITION
                | WM_IME_ENDCOMPOSITION
                | WM_ENABLE
                | WM_KILLFOCUS
                | WM_CANCELMODE
                | WM_CHAR
                | WM_SETTEXT
                | WM_UNDO
                | WM_CUT
                | WM_CLEAR
                | WM_KEYDOWN
                | WM_LBUTTONDOWN
                | WM_LBUTTONUP
                | WM_WINDOWPOSCHANGED
                | WM_SHOWWINDOW
        ) || matches!(
            message,
            windows::Win32::UI::Controls::EM_REPLACESEL
                | windows::Win32::UI::Controls::EM_SETSEL
                | windows::Win32::UI::Controls::EM_SETREADONLY
        ) || (message == WM_MOUSEMOVE && wp.0 & 1 != 0))
    {
        number_paste::invalidate(hwnd);
    }
    if composing & 3 == 2 && message == WM_PASTE {
        number_paste::paste(hwnd);
        return LRESULT(0);
    }
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
    }
    if message == WM_IME_STARTCOMPOSITION {
        let _ = SetWindowSubclass(hwnd, Some(procedure), ID, composing | 1);
        if let Ok(parent) = GetParent(hwnd) {
            SendMessageW(
                parent,
                COMPOSITION_START,
                None,
                Some(LPARAM(hwnd.0 as isize)),
            );
        }
    }
    // 组合中间态不预筛；Session 继续负责不可预判路径及最终草稿边界。
    if composing & 3 == 2 && number_preflight::rejects(hwnd, message, wp, lp) {
        return LRESULT(0);
    }
    if composing & 1 == 0
        && composing & 2 != 0
        && message == WM_KEYDOWN
        && matches!(wp.0, key if key == VK_UP.0 as usize || key == VK_DOWN.0 as usize)
        && GetFocus() == hwnd
        && IsWindowEnabled(hwnd).as_bool()
        && [VK_CONTROL, VK_MENU, VK_SHIFT]
            .iter()
            .all(|key| GetKeyState(key.0 as i32) >= 0)
    {
        if let Ok(parent) = GetParent(hwnd) {
            SendMessageW(
                parent,
                if wp.0 == VK_UP.0 as usize {
                    STEP_UP
                } else {
                    STEP_DOWN
                },
                None,
                Some(LPARAM(hwnd.0 as isize)),
            );
        }
        return LRESULT(0);
    }
    if composing & 1 == 0
        && GetWindowLongW(hwnd, GWL_STYLE) as u32 & ES_MULTILINE as u32 == 0
        && wp.0 == VK_RETURN.0 as usize
        && matches!(message, WM_KEYDOWN | WM_CHAR | WM_KEYUP)
    {
        if matches!(message, WM_KEYDOWN | WM_KEYUP) {
            if let Ok(parent) = GetParent(hwnd) {
                let mut flags = u32::from(message == WM_KEYUP);
                if message == WM_KEYDOWN && lp.0 as usize & (1 << 30) != 0 {
                    flags |= 2;
                }
                for (key, bit) in [
                    (VK_CONTROL, 4),
                    (VK_SHIFT, 8),
                    (VK_MENU, 16),
                    (VK_LWIN, 32),
                    (VK_RWIN, 32),
                ] {
                    if GetKeyState(key.0 as i32) < 0 {
                        flags |= bit;
                    }
                }
                if lp.0 as usize & (1 << 24) != 0 {
                    flags |= 64;
                }
                if SendMessageW(
                    parent,
                    ENTER_KEY,
                    Some(WPARAM(flags as usize)),
                    Some(LPARAM(hwnd.0 as isize)),
                )
                .0 != 0
                {
                    return LRESULT(0);
                }
            }
        }
        if message == WM_KEYDOWN && lp.0 as usize & (1 << 30) == 0 {
            if let Ok(parent) = GetParent(hwnd) {
                SendMessageW(parent, COMMIT, None, Some(LPARAM(hwnd.0 as isize)));
            }
        }
        return LRESULT(0);
    }
    if composing & 1 == 0
        && matches!(message, WM_KEYDOWN | WM_CHAR | WM_KEYUP)
        && wp.0 == VK_TAB.0 as usize
    {
        if message == WM_KEYDOWN && lp.0 as usize & (1 << 30) == 0 {
            if let Ok(parent) = GetParent(hwnd) {
                SendMessageW(
                    parent,
                    NAVIGATE,
                    Some(WPARAM(usize::from(GetKeyState(VK_SHIFT.0 as i32) < 0))),
                    Some(LPARAM(hwnd.0 as isize)),
                );
            }
        }
        return LRESULT(0);
    }
    // 一次替换可能发出删除/插入两次 EN_CHANGE；只在外层编辑命令结束后读取最终值。
    let editing = matches!(
        message,
        WM_CHAR | WM_PASTE | WM_CUT | WM_CLEAR | WM_UNDO | WM_SETTEXT
    ) || message == windows::Win32::UI::Controls::EM_REPLACESEL
        || (message == WM_KEYDOWN && wp.0 == VK_DELETE.0 as usize);
    if editing {
        if let Ok(parent) = GetParent(hwnd) {
            SendMessageW(parent, EDIT_BEGIN, None, Some(LPARAM(hwnd.0 as isize)));
        }
    }
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if editing && IsWindow(Some(hwnd)).as_bool() {
        if let Ok(parent) = GetParent(hwnd) {
            SendMessageW(parent, EDIT_END, None, Some(LPARAM(hwnd.0 as isize)));
        }
    }
    // 先让 EDIT 完成最终字符写入，再读取草稿；保留系统默认 IME 处理。
    if message == WM_IME_ENDCOMPOSITION && IsWindow(Some(hwnd)).as_bool() {
        let _ = SetWindowSubclass(hwnd, Some(procedure), ID, composing & !1);
        if let Ok(parent) = GetParent(hwnd) {
            SendMessageW(parent, COMPOSITION_END, None, Some(LPARAM(hwnd.0 as isize)));
        }
    }
    result
}
