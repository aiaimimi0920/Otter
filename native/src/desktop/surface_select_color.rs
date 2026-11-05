//! HWND 拥有已解析的声明文字色；绘制只合成颜色，不解析 CSS、不保存业务状态。
use super::*;
use crate::desktop::surface_control_color::{self as owner, Palette};
use crate::surface_color::Color;
pub(super) unsafe fn clear(hwnd: HWND) {
    owner::clear(hwnd);
}
pub(super) fn update(hwnd: HWND, value: Option<Color>) -> Result<()> {
    if !owner::update(
        hwnd,
        Palette {
            foreground: value,
            background: None,
        },
    )? {
        return Ok(());
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let mut info = windows::Win32::UI::Controls::COMBOBOXINFO {
            cbSize: std::mem::size_of::<windows::Win32::UI::Controls::COMBOBOXINFO>() as u32,
            ..Default::default()
        };
        if windows::Win32::UI::Controls::GetComboBoxInfo(hwnd, &mut info).is_ok() {
            let _ = InvalidateRect(Some(info.hwndList), None, false);
        }
        Ok(())
    }
}
pub(super) unsafe fn ink(hwnd: HWND, background: COLORREF, fallback: COLORREF) -> COLORREF {
    let Some(color) = owner::current(hwnd).foreground else {
        return fallback;
    };
    // GDI 不接受文字 alpha；相对本次实际背景合成，透明文字与 hover 背景保持一致。
    owner::blend(Some(color), background)
}
