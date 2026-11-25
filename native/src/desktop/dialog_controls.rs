//! Compact dialogs share Neuro rendering; preserve system editing, IME, keyboard, and accessibility, taking over appearance only.
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{Controls::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
};

pub(super) fn color(rgb: u32) -> COLORREF {
    COLORREF(((rgb & 255) << 16) | (rgb & 0xff00) | (rgb >> 16))
}
pub(super) unsafe fn button(hwnd: HWND) -> Result<()> {
    neuro_mini_ui::prepare_button(hwnd)
}
pub(super) unsafe fn edit(hwnd: HWND) -> Result<()> {
    SetWindowSubclass(hwnd, Some(edit_proc), 0x4f74, 0).ok()
}
pub(super) unsafe fn background(message: u32, wp: WPARAM, lp: LPARAM) -> Option<isize> {
    if !matches!(
        message,
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX
    ) {
        return None;
    }
    let p = neuro_mini_ui::Palette::default();
    let dc = HDC(wp.0 as *mut _);
    let mut class = [0u16; 16];
    let len = GetClassNameW(HWND(lp.0 as *mut _), &mut class) as usize;
    let editing = class[..len].iter().copied().eq("Edit".encode_utf16());
    let bg = color(if editing { p.control } else { p.surface });
    let _ = SetTextColor(dc, color(p.text));
    let _ = SetBkColor(dc, bg);
    SetDCBrushColor(dc, bg);
    Some(GetStockObject(DC_BRUSH).0 as isize)
}
pub(super) unsafe fn draw_button(item: &DRAWITEMSTRUCT, active: bool) {
    paint_button(item, active, true);
}
pub(super) unsafe fn draw_action_button(item: &DRAWITEMSTRUCT) {
    paint_button(item, false, false);
}
unsafe fn paint_button(item: &DRAWITEMSTRUCT, active: bool, outline: bool) {
    let p = neuro_mini_ui::Palette::default();
    let mut cursor = POINT::default();
    let mut screen = RECT::default();
    let hot = GetCursorPos(&mut cursor).is_ok()
        && GetWindowRect(item.hwndItem, &mut screen).is_ok()
        && cursor.x >= screen.left
        && cursor.x < screen.right
        && cursor.y >= screen.top
        && cursor.y < screen.bottom;
    let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
    let focus = item.itemState.0 & ODS_FOCUS.0 != 0;
    SetDCBrushColor(
        item.hDC,
        color(
            if hot || active || (!outline && focus) || item.itemState.0 & ODS_SELECTED.0 != 0 {
                p.control
            } else {
                p.surface
            },
        ),
    );
    FillRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    let ink = color(if disabled {
        p.muted
    } else if outline && (active || focus) {
        p.signal
    } else {
        p.text
    });
    let _ = SetTextColor(item.hDC, ink);
    let _ = SetBkMode(item.hDC, TRANSPARENT);
    if outline && (focus || active) {
        SetDCBrushColor(item.hDC, ink);
        FrameRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    let mut text = [0u16; 128];
    let len = GetWindowTextW(item.hwndItem, &mut text) as usize;
    let mut bounds = item.rcItem;
    DrawTextW(
        item.hDC,
        &mut text[..len],
        &mut bounds,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}
unsafe fn frame(hwnd: HWND) {
    let dc = GetWindowDC(Some(hwnd));
    if dc.is_invalid() {
        return;
    }
    let mut rect = RECT::default();
    if GetWindowRect(hwnd, &mut rect).is_ok() {
        rect.right -= rect.left;
        rect.bottom -= rect.top;
        rect.left = 0;
        rect.top = 0;
        let p = neuro_mini_ui::Palette::default();
        SetDCBrushColor(
            dc,
            color(if GetFocus() == hwnd {
                p.signal
            } else {
                p.muted
            }),
        );
        FrameRect(dc, &rect, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    ReleaseDC(Some(hwnd), dc);
}
unsafe extern "system" fn edit_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    if msg == WM_NCPAINT {
        frame(hwnd);
        return LRESULT(0);
    }
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if matches!(msg, WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE) && IsWindow(Some(hwnd)).as_bool() {
        frame(hwnd);
    }
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(edit_proc), id);
    }
    result
}
