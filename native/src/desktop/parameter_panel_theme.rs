//! Neuro 语义映射；只借用系统 DC_BRUSH/字体，不常驻分配 GDI 对象。
use super::*;
#[path = "parameter_panel_options_paint.rs"]
pub(super) mod options;
pub(in crate::desktop) const PANEL: u32 = 0x18120e;
const CONTROL: u32 = 0x201711;
const TEXT: u32 = 0xeff8f7;
const MUTED: u32 = 0x9f9a92;
const YELLOW: u32 = 0x38ffd9;
pub(in crate::desktop) unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    SetDCBrushColor(dc, COLORREF(color));
    FillRect(dc, rect, HBRUSH(GetStockObject(DC_BRUSH).0));
}
pub(in crate::desktop) unsafe fn colors(msg: u32, wp: WPARAM, lp: LPARAM, error: bool) -> LRESULT {
    let dc = HDC(wp.0 as *mut _);
    let focus = msg == WM_CTLCOLOREDIT && GetFocus().0 as isize == lp.0;
    let background = if focus {
        0xf1f5f3
    } else if msg == WM_CTLCOLORSTATIC {
        PANEL
    } else {
        CONTROL
    };
    SetTextColor(
        dc,
        COLORREF(if error {
            0x5e3ff4
        } else if focus {
            0x2b2520
        } else {
            TEXT
        }),
    );
    SetBkColor(dc, COLORREF(background));
    SetDCBrushColor(dc, COLORREF(background));
    LRESULT(GetStockObject(DC_BRUSH).0 as isize)
}
pub(in crate::desktop) unsafe fn button(item: &DRAWITEMSTRUCT) {
    let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
    let font = SendMessageW(item.hwndItem, WM_GETFONT, None, None);
    let old = SelectObject(item.hDC, HGDIOBJ(font.0 as *mut _));
    fill(
        item.hDC,
        &item.rcItem,
        if item.itemState.0 & ODS_SELECTED.0 != 0 {
            0x2c2218
        } else {
            CONTROL
        },
    );
    SetTextColor(item.hDC, COLORREF(if disabled { MUTED } else { TEXT }));
    SetBkMode(item.hDC, TRANSPARENT);
    let mut text = [0u16; 260];
    let count = GetWindowTextW(item.hwndItem, &mut text) as usize;
    let mut rect = item.rcItem;
    rect.left += 6;
    rect.right -= 6;
    DrawTextW(
        item.hDC,
        &mut text[..count],
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
    if item.itemState.0 & ODS_FOCUS.0 != 0 {
        SetDCBrushColor(item.hDC, COLORREF(YELLOW));
        FrameRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    SelectObject(item.hDC, old);
}
