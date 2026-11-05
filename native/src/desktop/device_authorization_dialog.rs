//! 服务地址草稿只在确定时校验；标准 Win32 对话框提供 Tab、Enter、Escape 和焦点恢复。
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "device_authorization_dialog_template.rs"]
mod layout;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
struct Input<'a> {
    initial: &'a str,
    result: Option<String>,
    art: bool,
}
pub(super) unsafe fn show(owner: HWND, initial: &str) -> Option<String> {
    show_input(owner, initial, false)
}
pub(super) unsafe fn show_art(owner: HWND) -> Option<String> {
    show_input(owner, "", true)
}
unsafe fn show_input(owner: HWND, initial: &str, art: bool) -> Option<String> {
    let template = layout::template(art);
    let mut input = Input {
        initial,
        result: None,
        art,
    };
    if DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&mut input as *mut _ as isize),
    ) == -1
    {
        eprintln!(
            "Authorization dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        let input = &*(lp.0 as *const Input<'_>);
        let _ = SetDlgItemTextW(hwnd, 101, &HSTRING::from(input.initial));
        if let Ok(edit) = GetDlgItem(Some(hwnd), 101) {
            SendMessageW(
                edit,
                0x00C5,
                Some(WPARAM(if input.art { 256 } else { 2048 })),
                Some(LPARAM(0)),
            );
        }
        return 1;
    }
    if message == WM_CLOSE || (message == WM_COMMAND && wp.0 & 0xffff == IDCANCEL.0 as usize) {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if message != WM_COMMAND || wp.0 & 0xffff != IDOK.0 as usize {
        return 0;
    }
    let input = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *mut Input<'_>;
    if input.is_null() {
        return 0;
    }
    let mut buffer = [0u16; 2050];
    let count = GetDlgItemTextW(hwnd, 101, &mut buffer) as usize;
    let value = String::from_utf16_lossy(&buffer[..count]);
    let validated = if (*input).art {
        crate::live_relay_viewer_identity::identifier(&value).then_some(value.clone())
    } else {
        crate::device_session_attempt::canonical_origin(&value).ok()
    };
    if let Some(origin) = validated {
        (*input).result = Some(origin);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            103,
            &HSTRING::from(if (*input).art {
                "请输入有效的 Art ID。"
            } else {
                "请输入 HTTPS 服务地址；不能包含账号、路径、查询或片段。"
            }),
        );
    }
    1
}
