//! 固定高度 owner-draw 仅接管像素；HASSTRINGS 保留系统键盘、索引和可访问性。
use super::*;
use crate::desktop::surface_control_colors as colors;
use windows::Win32::UI::{Controls::*, HiDpi::GetDpiForWindow, Shell::*};
const ID: usize = 0x4f54534c;
struct Saved(HDC, i32);
impl Drop for Saved {
    fn drop(&mut self) {
        unsafe {
            let _ = RestoreDC(self.0, self.1);
        }
    }
}
pub(super) fn install(parent: HWND) -> Result<()> {
    // 父 HWND 生命周期内至多一个无堆状态 hook；多个 Select 共享，销毁父窗口时解除。
    if unsafe { SetWindowSubclass(parent, Some(procedure), ID, 0) }.as_bool() {
        Ok(())
    } else {
        Err(Error::from_win32())
    }
}
pub(super) fn resize(hwnd: HWND) -> Result<()> {
    unsafe {
        let dc = GetDC(Some(hwnd));
        if dc.is_invalid() {
            return Err(Error::from_win32());
        }
        let font = SendMessageW(hwnd, WM_GETFONT, None, None).0;
        let old = SelectObject(dc, HGDIOBJ(font as *mut _));
        let mut metrics = TEXTMETRICW::default();
        let measured = GetTextMetricsW(dc, &mut metrics).as_bool();
        SelectObject(dc, old);
        ReleaseDC(Some(hwnd), dc);
        if !measured {
            return Err(Error::from_win32());
        }
        let pad = (4 * GetDpiForWindow(hwnd).clamp(48, 768)).div_ceil(96) as i32;
        let height = metrics.tmHeight.saturating_add(pad).clamp(1, 32768);
        for index in [0, usize::MAX] {
            // 相同样式重排不能设置高度，否则系统可能关闭已展开的列表。
            if SendMessageW(hwnd, CB_GETITEMHEIGHT, Some(WPARAM(index)), None).0 != height as isize
                && SendMessageW(
                    hwnd,
                    CB_SETITEMHEIGHT,
                    Some(WPARAM(index)),
                    Some(LPARAM(height as isize)),
                )
                .0 == CB_ERR as isize
            {
                return Err(Error::new(E_FAIL, "Select item height update failed"));
            }
        }
        Ok(())
    }
}
unsafe fn draw(item: &DRAWITEMSTRUCT) -> bool {
    let dc = item.hDC;
    let saved = SaveDC(dc);
    if saved == 0 {
        return false;
    }
    let _saved = Saved(dc, saved);
    let selected = item.itemState.0 & ODS_SELECTED.0 != 0;
    let disabled =
        item.itemState.0 & ODS_DISABLED.0 != 0 || !IsWindowEnabled(item.hwndItem).as_bool();
    let themed = colors::enabled();
    let (background, ink) = if themed {
        (
            if selected {
                colors::FOCUS_SURFACE
            } else {
                colors::BACKGROUND
            },
            if disabled && selected {
                colors::FOCUS_MUTED
            } else if disabled {
                colors::MUTED
            } else if selected {
                colors::FOCUS_INK
            } else {
                color::ink(item.hwndItem, colors::BACKGROUND, colors::TEXT)
            },
        )
    } else {
        // owner-draw 高对比度不能依靠默认过程补画，必须显式使用系统配色。
        (
            COLORREF(GetSysColor(if selected {
                COLOR_HIGHLIGHT
            } else {
                COLOR_WINDOW
            })),
            COLORREF(GetSysColor(if disabled {
                COLOR_GRAYTEXT
            } else if selected {
                COLOR_HIGHLIGHTTEXT
            } else {
                COLOR_WINDOWTEXT
            })),
        )
    };
    let rect = item.rcItem;
    IntersectClipRect(dc, rect.left, rect.top, rect.right, rect.bottom);
    SetDCBrushColor(dc, background);
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    FillRect(dc, &rect, brush);
    SetTextColor(dc, ink);
    SetBkMode(dc, TRANSPARENT);
    let font = SendMessageW(item.hwndItem, WM_GETFONT, None, None).0;
    if font != 0 {
        SelectObject(dc, HGDIOBJ(font as *mut _));
    }
    let dpi = GetDpiForWindow(item.hwndItem).clamp(48, 768);
    let pad = (3 * dpi).div_ceil(96) as i32;
    if let Some(mut text) = text::at(item.hwndItem, item.itemID as usize) {
        let mut text_rect = rect;
        text_rect.left += pad;
        text_rect.right -= pad;
        if !text.is_empty() {
            DrawTextW(
                dc,
                &mut text,
                &mut text_rect,
                DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
        }
    }
    if item.itemState.0 & ODS_FOCUS.0 != 0 && item.itemState.0 & ODS_NOFOCUSRECT.0 == 0 {
        if themed {
            let mut border = rect;
            SetDCBrushColor(dc, colors::FOCUS);
            for _ in 0..(2 * dpi).div_ceil(96) {
                if border.right <= border.left || border.bottom <= border.top {
                    break;
                }
                FrameRect(dc, &border, brush);
                border.left += 1;
                border.top += 1;
                border.right -= 1;
                border.bottom -= 1;
            }
        } else {
            let _ = DrawFocusRect(dc, &rect);
        }
    }
    true
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if message == WM_DRAWITEM && lp.0 != 0 {
        // WM_DRAWITEM 是同线程系统回调；只为属于该父窗口的自有 Select 绘制。
        let item = &*(lp.0 as *const DRAWITEMSTRUCT);
        if item.CtlType == ODT_COMBOBOX
            && !GetPropW(item.hwndItem, OWNER).is_invalid()
            && GetParent(item.hwndItem).ok() == Some(hwnd)
            && draw(item)
        {
            return LRESULT(1);
        }
    }
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
