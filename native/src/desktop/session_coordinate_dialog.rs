//! 旧会话坐标输入：空默认值、有限长度、同步模态 owner；不查询当前显示器猜历史值。
use crate::session_placement::Space;
use windows::{
    core::HSTRING,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
struct Input {
    result: Option<Space>,
}

pub(super) unsafe fn show(owner: HWND) -> Option<Space> {
    let mut input = Input { result: None };
    let template = template();
    if DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&mut input as *mut _ as isize),
    ) == -1
    {
        eprintln!(
            "Session coordinate dialog failed: {}",
            windows::core::Error::from_win32()
        );
    }
    input.result
}

fn parse(x: &str, y: &str, scale: &str) -> Option<Space> {
    if [x, y, scale].iter().any(|text| text.len() > 64) {
        return None;
    }
    crate::session_coordinates::select(
        Some(Space {
            origin: (x.trim().parse().ok()?, y.trim().parse().ok()?),
            scale: scale.trim().parse().ok()?,
        }),
        None,
    )
    .ok()
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        for id in [101, 102, 103] {
            if let Ok(edit) = GetDlgItem(Some(hwnd), id) {
                SendMessageW(edit, 0x00C5, Some(WPARAM(64)), Some(LPARAM(0)));
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
    let texts: Vec<_> = [101, 102, 103]
        .into_iter()
        .map(|id| {
            let mut buffer = [0u16; 66];
            let count = GetDlgItemTextW(hwnd, id, &mut buffer) as usize;
            String::from_utf16_lossy(&buffer[..count])
        })
        .collect();
    if let Some(space) = parse(&texts[0], &texts[1], &texts[2]) {
        (*input).result = Some(space);
        let _ = EndDialog(hwnd, 1);
    } else {
        let _ = SetDlgItemTextW(
            hwnd,
            104,
            &HSTRING::from("原点须为整数；缩放须为有效正数（例如 1.5）。"),
        );
    }
    1
}

fn template() -> Vec<u32> {
    fn string(data: &mut Vec<u16>, text: &str) {
        data.extend(text.encode_utf16());
        data.push(0);
    }
    fn dword(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    dword(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, 0);
    data.extend([9, 0, 0, 280, 142, 0, 0]);
    string(&mut data, "旧工作区坐标");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (201, 0x82, 0, 10, 12, 68, 14, "原点 X"),
        (
            101,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32,
            82,
            10,
            188,
            16,
            "",
        ),
        (202, 0x82, 0, 10, 36, 68, 14, "原点 Y"),
        (
            102,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32,
            82,
            34,
            188,
            16,
            "",
        ),
        (203, 0x82, 0, 10, 60, 68, 14, "缩放"),
        (
            103,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32,
            82,
            58,
            188,
            16,
            "",
        ),
        (
            104,
            0x82,
            0,
            10,
            82,
            260,
            28,
            "填写原工作区的坐标原点和缩放；不确定时请取消。",
        ),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            150,
            116,
            55,
            18,
            "确定",
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0,
            215,
            116,
            55,
            18,
            "取消",
        ),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        string(&mut data, text);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|pair| u32::from(pair[0]) | (u32::from(pair[1]) << 16))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_fields_are_required_and_bounded() {
        let value = parse(" -1920 ", "-200", "1.5").unwrap();
        assert_eq!(value.origin, (-1920, -200));
        assert_eq!(value.scale, 1.5);
        for fields in [
            ("", "0", "1"),
            ("0", "", "1"),
            ("0", "0", ""),
            ("0.5", "0", "1"),
            ("2147483648", "0", "1"),
            ("0", "0", "NaN"),
            ("0", "0", "0"),
            ("0", "0", "-1"),
            ("0", "0", "1e309"),
        ] {
            assert!(parse(fields.0, fields.1, fields.2).is_none());
        }
        assert!(parse(&"0".repeat(65), "0", "1").is_none());
        assert!(!template().is_empty());
    }
}
