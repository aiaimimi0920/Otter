//! The EDIT font subclass owns only destruction order; reuse the control-font module for parsing, caching, and HFONT ownership.
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
        // Install cleanup before transferring any HFONT; reinstalling the same ID adds no owner.
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
        drop(font); // Release only after default EDIT and all other subclasses finish their final paint/destruction.
        result
    } else {
        DefSubclassProc(hwnd, message, wp, lp)
    }
}
