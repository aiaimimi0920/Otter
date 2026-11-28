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
    let w = (item.rcItem.right - item.rcItem.left).max(1);
    let h = (item.rcItem.bottom - item.rcItem.top).max(1);
    let mem_dc = CreateCompatibleDC(Some(item.hDC));
    let mem_bmp = if !mem_dc.is_invalid() {
        CreateCompatibleBitmap(item.hDC, w, h)
    } else {
        HBITMAP::default()
    };
    let buffered = !mem_dc.is_invalid() && !mem_bmp.is_invalid();
    let (dc, rc, old_bmp) = if buffered {
        let old = SelectObject(mem_dc, HGDIOBJ(mem_bmp.0));
        (
            mem_dc,
            RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            },
            Some(old),
        )
    } else {
        (item.hDC, item.rcItem, None)
    };
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
        dc,
        color(
            if hot || active || (!outline && focus) || item.itemState.0 & ODS_SELECTED.0 != 0 {
                p.control
            } else {
                p.surface
            },
        ),
    );
    FillRect(dc, &rc, HBRUSH(GetStockObject(DC_BRUSH).0));
    let ink = color(if disabled {
        p.muted
    } else if outline && (active || focus) {
        p.signal
    } else {
        p.text
    });
    let _ = SetTextColor(dc, ink);
    let _ = SetBkMode(dc, TRANSPARENT);
    if outline && (focus || active) {
        SetDCBrushColor(dc, ink);
        FrameRect(dc, &rc, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    let font_msg = SendMessageW(item.hwndItem, WM_GETFONT, None, None);
    let font = if font_msg.0 != 0 {
        HGDIOBJ(font_msg.0 as *mut _)
    } else {
        GetStockObject(DEFAULT_GUI_FONT)
    };
    let old_font = SelectObject(dc, font);
    let mut text = [0u16; 128];
    let len = GetWindowTextW(item.hwndItem, &mut text) as usize;
    let mut bounds = rc;
    DrawTextW(
        dc,
        &mut text[..len],
        &mut bounds,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    let _ = SelectObject(dc, old_font);
    if let Some(old) = old_bmp {
        let _ = BitBlt(
            item.hDC,
            item.rcItem.left,
            item.rcItem.top,
            w,
            h,
            Some(mem_dc),
            0,
            0,
            SRCCOPY,
        );
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteObject(HGDIOBJ(mem_bmp.0));
    }
    if !mem_dc.is_invalid() {
        let _ = DeleteDC(mem_dc);
    }
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
