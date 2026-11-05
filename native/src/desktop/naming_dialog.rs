//! 有界四字段原生命名编辑器；对话框只持有草稿，取消不修改共享设置或文件。
use crate::file_naming_settings::Settings;
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "naming_dialog_template.rs"]
mod layout;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
const LIMIT: usize = 4096;
struct Input {
    initial: Settings,
    result: Option<Settings>,
}

pub(super) unsafe fn show(owner: HWND, initial: Settings) -> Option<Settings> {
    let mut input = Input {
        initial,
        result: None,
    };
    let template = layout::template();
    if DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&mut input as *mut _ as isize),
    ) == -1
    {
        eprintln!(
            "Naming dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result
}

fn parse(initial: &Settings, fields: &[String; 4]) -> Option<Settings> {
    if fields.iter().any(|value| value.len() > LIMIT) {
        return None;
    }
    let mut next = initial.clone();
    next.sticker_save_pattern.clone_from(&fields[0]);
    next.drag_export_pattern.clone_from(&fields[1]);
    next.clipboard_file_pattern.clone_from(&fields[2]);
    next.title_max_length = fields[3].trim().parse().ok()?;
    next.validate().ok()?;
    Some(next)
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        let input = &*(lp.0 as *const Input);
        let values = [
            &input.initial.sticker_save_pattern,
            &input.initial.drag_export_pattern,
            &input.initial.clipboard_file_pattern,
            &input.initial.title_max_length.to_string(),
        ];
        for (index, value) in values.into_iter().enumerate() {
            let id = 101 + index as i32;
            let _ = SetDlgItemTextW(hwnd, id, &HSTRING::from(value));
            if let Ok(edit) = GetDlgItem(Some(hwnd), id) {
                SendMessageW(
                    edit,
                    0x00C5,
                    Some(WPARAM(if index == 3 { 3 } else { LIMIT })),
                    Some(LPARAM(0)),
                );
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
    // 多取一个字符以拒绝超限输入，不把截断后的前缀当作合法草稿。
    let fields = std::array::from_fn(|index| {
        let mut buffer = [0u16; LIMIT + 2];
        let count = GetDlgItemTextW(hwnd, 101 + index as i32, &mut buffer) as usize;
        String::from_utf16_lossy(&buffer[..count])
    });
    if let Some(next) = parse(&(*input).initial, &fields) {
        (*input).result = Some(next);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            105,
            &HSTRING::from("模板须使用有效占位符且不超过 4096 字节；标题长度须为 1–240 的整数。"),
        );
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields() -> [String; 4] {
        ["保存_{title}", "拖出_{shortId}", "剪贴板_{kind}", "12"].map(str::to_owned)
    }
    #[test]
    fn all_fields_are_validated_together_and_policy_is_retained() {
        let original = Settings::default();
        let next = parse(&original, &fields()).unwrap();
        assert_eq!(next.sticker_save_pattern, "保存_{title}");
        assert_eq!(next.drag_export_pattern, "拖出_{shortId}");
        assert_eq!(next.clipboard_file_pattern, "剪贴板_{kind}");
        assert_eq!(next.title_max_length, 12);
        assert_eq!(next.collision_policy, original.collision_policy);
        assert_eq!(original, Settings::default());
        assert!(!layout::template().is_empty());
    }
    #[test]
    fn invalid_fields_never_produce_partial_settings() {
        for index in 0..3 {
            for invalid in [
                "{unknown}".into(),
                "x".repeat(LIMIT + 1),
                "中".repeat(LIMIT / 3 + 1),
            ] {
                let mut values = fields();
                values[index] = invalid;
                assert!(parse(&Settings::default(), &values).is_none());
            }
        }
        for invalid in ["", "0", "241", "1.5", "-1", "NaN", "18446744073709551616"] {
            let mut values = fields();
            values[3] = invalid.into();
            assert!(parse(&Settings::default(), &values).is_none());
        }
    }
}
