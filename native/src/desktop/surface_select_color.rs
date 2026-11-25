//! HWND-owned parsed declared text colors; painting only composites colors, without CSS parsing or business state.
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
    // GDI does not accept text alpha; composite against this actual background so transparent text matches hover backgrounds.
    owner::blend(Some(color), background)
}
