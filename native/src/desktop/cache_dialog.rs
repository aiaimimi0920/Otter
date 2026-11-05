//! 两字段 Win32 输入；以整数精确保留字节配额，不把非整 MiB 的配置值偷偷四舍五入。
use crate::cache_policy::Policy;
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "cache_dialog_template.rs"]
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
            "Cache dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result
}
fn parse(bytes: &str, days: &str) -> Option<Policy> {
    if bytes.len() > 11 || days.len() > 4 {
        return None;
    }
    Policy {
        temp_cache_max_bytes: bytes.trim().parse().ok()?,
        temp_cache_retention_days: days.trim().parse().ok()?,
    }
    .validate()
    .ok()
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        let input = &*(lp.0 as *const Input);
        for (id, limit, value) in [
            (101, 11, input.initial.temp_cache_max_bytes.to_string()),
            (102, 4, input.initial.temp_cache_retention_days.to_string()),
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
    let mut buffer = [0u16; 14];
    let count = GetDlgItemTextW(hwnd, 101, &mut buffer) as usize;
    let bytes = String::from_utf16_lossy(&buffer[..count]);
    let count = GetDlgItemTextW(hwnd, 102, &mut buffer) as usize;
    let days = String::from_utf16_lossy(&buffer[..count]);
    if let Some(policy) = parse(&bytes, &days) {
        (*input).result = Some(policy);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            103,
            &HSTRING::from("容量须为 0 或 33554432–17179869184 字节；天数须为 0–3650。"),
        );
    }
    1
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_integer_ranges_zero_and_non_mebibyte_values_are_preserved() {
        assert_eq!(
            parse("33554433", "30").unwrap().temp_cache_max_bytes,
            33554433
        );
        assert_eq!(parse("0", "0").unwrap().temp_cache_retention_days, 0);
        assert!(parse("17179869184", "3650").is_some());
        for (bytes, days) in [
            ("", "7"),
            ("1", "7"),
            ("33554432.5", "7"),
            ("-1", "7"),
            ("17179869185", "7"),
            ("0", "3651"),
            ("0", "1.5"),
            ("0", "-1"),
        ] {
            assert!(parse(bytes, days).is_none());
        }
        assert!(!layout::template().is_empty());
    }
}
