//! 回收站两字段原生模态；输入有界、严格整数，0 只关闭对应软限制。
use crate::session_library::recycle::policy::Policy;
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "recycle_dialog_template.rs"]
mod layout;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
struct Input {
    initial: Policy,
    result: Option<Policy>,
}
pub(super) unsafe fn show(owner: HWND, initial: Policy) -> Option<Policy> {
    let template = layout::template();
    let mut input = Input {
        initial,
        result: None,
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
            "Recycle dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result
}
fn parse(entries: &str, days: &str) -> Option<Policy> {
    if entries.len() > 3 || days.len() > 4 {
        return None;
    }
    Policy {
        recycle_bin_max_entries: entries.trim().parse().ok()?,
        recycle_bin_retention_days: days.trim().parse().ok()?,
    }
    .validate()
    .ok()
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        let input = &*(lp.0 as *const Input);
        for (id, limit, value) in [
            (101, 3, input.initial.recycle_bin_max_entries.to_string()),
            (102, 4, input.initial.recycle_bin_retention_days.to_string()),
        ] {
            let _ = SetDlgItemTextW(hwnd, id, &HSTRING::from(value));
            if let Ok(edit) = GetDlgItem(Some(hwnd), id) {
                SendMessageW(edit, 0x00C5, Some(WPARAM(limit)), Some(LPARAM(0)));
            }
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
    let input = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *mut Input;
    if input.is_null() {
        return 0;
    }
    let mut buffer = [0u16; 8];
    let count = GetDlgItemTextW(hwnd, 101, &mut buffer) as usize;
    let entries = String::from_utf16_lossy(&buffer[..count]);
    let count = GetDlgItemTextW(hwnd, 102, &mut buffer) as usize;
    let days = String::from_utf16_lossy(&buffer[..count]);
    if let Some(policy) = parse(&entries, &days) {
        (*input).result = Some(policy);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            103,
            &HSTRING::from("条数须为 0–500；保留天数须为 0–3650。"),
        );
    }
    1
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_ranges_and_zero_keep_policy_meaning() {
        assert_eq!(parse("15", "0"), Some(Policy::default()));
        assert_eq!(parse("0", "0").unwrap().recycle_bin_max_entries, 0);
        assert!(parse("500", "3650").is_some());
        for (entries, days) in [
            ("501", "0"),
            ("1", "3651"),
            ("-1", "0"),
            ("1", "-1"),
            ("1.5", "0"),
            ("1", "1.5"),
            ("", "0"),
            ("1", "999999"),
        ] {
            assert!(parse(entries, days).is_none());
        }
        assert!(!layout::template().is_empty());
    }
}
