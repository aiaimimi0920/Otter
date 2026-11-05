//! EDIT 声明配色及销毁接线；不依赖可重入期间暂时移出的 Host，不持有业务指针。
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
    // 颜色不是授权凭证：仍校验实际父窗口和 EDIT owner marker，仅接管呈现。
    if dc.is_invalid()
        || GetParent(hwnd).ok() != Some(parent)
        || GetPropW(hwnd, OWNER).is_invalid()
        || !theme::enabled()
    {
        return None;
    }
    let palette = owner::current(hwnd); // 复制后不跨 Win32 调用借用 property。
    if palette == owner::Palette::default() {
        return None;
    }
    // Classic EDIT 是不透明 GDI 子窗口；alpha 背景暂以控件底色预合成，非任意父像素透明。
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
