//! 只接管标准下拉框的项像素；系统仍拥有字符串、索引、键盘和可访问性。
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
const ID: usize = 0x4f54504f;

pub(in crate::desktop) unsafe fn resize(hwnd: HWND) -> Result<()> {
    SetWindowSubclass(hwnd, Some(procedure), ID, 0).ok()?;
    let height = (24 * GetDpiForWindow(hwnd).max(96)).div_ceil(96) as isize;
    for index in [0, usize::MAX] {
        if SendMessageW(hwnd, CB_GETITEMHEIGHT, Some(WPARAM(index)), None).0 != height
            && SendMessageW(
                hwnd,
                CB_SETITEMHEIGHT,
                Some(WPARAM(index)),
                Some(LPARAM(height)),
            )
            .0 == CB_ERR as isize
        {
            return Err(Error::new(E_FAIL, "Cannot size parameter options"));
        }
    }
    Ok(())
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
    } else if let Some(result) = desktop::surface_select_control::paint_standard(hwnd, msg, wp) {
        return result;
    }
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if matches!(msg, WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE) && IsWindow(Some(hwnd)).as_bool() {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    result
}

pub(in crate::desktop) unsafe fn draw(item: &DRAWITEMSTRUCT) {
    let saved = SaveDC(item.hDC);
    if saved == 0 {
        return;
    }
    let selected = item.itemState.0 & ODS_SELECTED.0 != 0;
    let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
    let background = if selected { 0xf1f5f3 } else { CONTROL };
    let ink = if selected && disabled {
        0x726b65
    } else if selected {
        0x2b2520
    } else if disabled {
        MUTED
    } else {
        TEXT
    };
    fill(item.hDC, &item.rcItem, background);
    SetTextColor(item.hDC, COLORREF(ink));
    SetBkMode(item.hDC, TRANSPARENT);
    let font = SendMessageW(item.hwndItem, WM_GETFONT, None, None);
    SelectObject(item.hDC, HGDIOBJ(font.0 as *mut _));
    if item.itemID != u32::MAX {
        let index = Some(WPARAM(item.itemID as usize));
        let length = SendMessageW(item.hwndItem, CB_GETLBTEXTLEN, index, None).0;
        // 插入时已清洗到128字符；UTF-16最多256单元，不按外部长度分配。
        if (0..=256).contains(&length) {
            let mut text = [0u16; 257];
            let actual = SendMessageW(
                item.hwndItem,
                CB_GETLBTEXT,
                index,
                Some(LPARAM(text.as_mut_ptr() as isize)),
            )
            .0;
            if actual == length {
                let mut rect = item.rcItem;
                let pad = (4 * GetDpiForWindow(item.hwndItem).max(96)).div_ceil(96) as i32;
                rect.left += pad;
                rect.right -= pad;
                DrawTextW(
                    item.hDC,
                    &mut text[..length as usize],
                    &mut rect,
                    DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
                );
            }
        }
    }
    if item.itemState.0 & ODS_FOCUS.0 != 0 && item.itemState.0 & ODS_NOFOCUSRECT.0 == 0 {
        SetDCBrushColor(item.hDC, COLORREF(YELLOW));
        FrameRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    let _ = RestoreDC(item.hDC, saved);
}
