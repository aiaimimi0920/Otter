//! EDIT 字体子类仅负责销毁顺序；解析、缓存和 HFONT 所有权复用控件字体模块。
use super::*;
use crate::desktop::surface_control_font;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
const ID: usize = 0x4f544546;
pub(super) fn update(
    hwnd: HWND,
    dpi: u32,
    style: &crate::surface_presentation::ControlFont,
) -> Result<()> {
    unsafe {
        // 在移交任何 HFONT 前安装清理；重复安装同一个 ID 不增加 owner。
        if !SetWindowSubclass(hwnd, Some(procedure), ID, 0).as_bool() {
            return Err(Error::from_win32());
        }
    }
    surface_control_font::update(hwnd, dpi, Some(style))
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        let font = surface_control_font::take(hwnd);
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
        let result = DefSubclassProc(hwnd, message, wp, lp);
        drop(font); // 默认 EDIT 及其余子类完成最后一次绘制/销毁后再释放。
        result
    } else {
        DefSubclassProc(hwnd, message, wp, lp)
    }
}
