//! COMBOBOX Tab 接缝；无会话裸指针，先收起列表再向 Surface 导航，其他键交系统。
use super::*;
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{GetKeyState, VK_SHIFT, VK_TAB},
    Shell::*,
};
const ID: usize = 0x4f545453;
pub(super) unsafe fn hovered(hwnd: HWND) -> bool {
    let mut state = 0;
    GetWindowSubclass(hwnd, Some(procedure), ID, Some(&mut state)).as_bool() && state != 0
}
pub(super) unsafe fn set_hover(hwnd: HWND, next: bool) -> bool {
    let mut state = 0;
    if !GetWindowSubclass(hwnd, Some(procedure), ID, Some(&mut state)).as_bool() {
        return false;
    }
    if (state != 0) == next {
        return true;
    }
    if !SetWindowSubclass(hwnd, Some(procedure), ID, usize::from(next)).as_bool() {
        return false;
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    true
}
pub(super) fn install(hwnd: HWND) -> Result<()> {
    if unsafe { SetWindowSubclass(hwnd, Some(procedure), ID, 0) }.as_bool() {
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
    _: usize,
) -> LRESULT {
    if crate::desktop::session_end::blocks_input(message) {
        return LRESULT(0);
    }
    if !GetPropW(hwnd, OWNER).is_invalid() {
        hover::update(hwnd, message, wp, lp);
        if let Some(result) = paint::message(hwnd, message, wp) {
            return result;
        }
    }
    if message == WM_CTLCOLORLISTBOX && !GetPropW(hwnd, OWNER).is_invalid() {
        let mut info = windows::Win32::UI::Controls::COMBOBOXINFO {
            cbSize: std::mem::size_of::<windows::Win32::UI::Controls::COMBOBOXINFO>() as u32,
            ..Default::default()
        };
        // 只接受当前 COMBOBOX 的实际 list HWND，不给任意消息附带的窗口套用配色。
        if windows::Win32::UI::Controls::GetComboBoxInfo(hwnd, &mut info).is_ok()
            && info.hwndList == HWND(lp.0 as *mut _)
        {
            if let Some(result) = crate::desktop::surface_control_colors::paint(HDC(wp.0 as *mut _))
            {
                return result;
            }
        }
    }
    if message == WM_NCDESTROY {
        hover::cancel_tracking(hwnd);
        let font = font::take(hwnd);
        color::clear(hwnd);
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
        let result = DefSubclassProc(hwnd, message, wp, lp);
        drop(font);
        return result;
    }
    if matches!(message, WM_KEYDOWN | WM_CHAR | WM_KEYUP) && wp.0 == VK_TAB.0 as usize {
        if message == WM_KEYDOWN
            && lp.0 as usize & (1 << 30) == 0
            && GetFocus() == hwnd
            && IsWindowEnabled(hwnd).as_bool()
        {
            let marker = GetPropW(hwnd, OWNER);
            SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
            // 收起会同步通知父窗口；重入关闭/撤销 owner 后不能继续向缓存句柄导航。
            if !marker.is_invalid()
                && IsWindow(Some(hwnd)).as_bool()
                && GetPropW(hwnd, OWNER) == marker
            {
                if let Ok(parent) = GetParent(hwnd) {
                    SendMessageW(
                        parent,
                        crate::desktop::surface_edit_messages::NAVIGATE,
                        Some(WPARAM(usize::from(GetKeyState(VK_SHIFT.0 as i32) < 0))),
                        Some(LPARAM(hwnd.0 as isize)),
                    );
                }
            }
        }
        return LRESULT(0);
    }
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if matches!(message, WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE) && IsWindow(Some(hwnd)).as_bool() {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    result
}
