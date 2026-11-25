//! EDIT declared-color and destruction wiring; independent of temporarily removed reentrant Hosts, with no business-state pointers.
use super::*;
use crate::desktop::{surface_control_color as owner, surface_control_colors as theme};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
const ID: usize = 0x4f544543;
impl Control {
    pub fn set_colors(&self, colors: [crate::surface_color::Color; 2]) -> Result<()> {
        let hwnd = self.hwnd()?;
        unsafe {
            if !SetWindowSubclass(hwnd, Some(procedure), ID, 0).as_bool() {
                return Err(Error::from_win32());
            }
        }
        if owner::update(
            hwnd,
            owner::Palette {
                foreground: Some(colors[0]),
                background: Some(colors[1]),
            },
        )? {
            unsafe {
                InvalidateRect(Some(self.hwnd()?), None, true).ok()?;
            }
        }
        Ok(())
    }
}
pub(in crate::desktop) unsafe fn paint(parent: HWND, hwnd: HWND, dc: HDC) -> Option<LRESULT> {
    // Colors are not authorization credentials: still verify the actual parent window and EDIT owner marker, taking over presentation only.
    if dc.is_invalid()
        || GetParent(hwnd).ok() != Some(parent)
        || GetPropW(hwnd, OWNER).is_invalid()
        || !theme::enabled()
    {
        return None;
    }
    let palette = owner::current(hwnd); // Copy first, retaining no property borrow across Win32 calls.
    if palette == owner::Palette::default() {
        return None;
    }
    // Classic EDIT is an opaque GDI child; alpha backgrounds currently precompose with the control base color, not arbitrary parent-pixel transparency.
    let background = owner::blend(palette.background, theme::BACKGROUND);
    let foreground = palette
        .foreground
        .map(|c| owner::blend(Some(c), background))
        .unwrap_or(theme::TEXT);
    SetTextColor(dc, foreground);
    SetBkColor(dc, background);
    SetDCBrushColor(dc, background);
    Some(LRESULT(GetStockObject(DC_BRUSH).0 as isize))
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
        owner::clear(hwnd);
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
