//! 仅接管 COMBOBOX 收起区域绘制；系统仍拥有选择、弹出、键盘、焦点与 UIA。
use super::*;
use crate::desktop::surface_control_colors as colors;
use windows::Win32::UI::{
    Controls::{GetComboBoxInfo, COMBOBOXINFO},
    HiDpi::GetDpiForWindow,
    Shell::DefSubclassProc,
};
struct Saved(HDC, i32);
impl Drop for Saved {
    fn drop(&mut self) {
        unsafe {
            let _ = RestoreDC(self.0, self.1);
        }
    }
}
unsafe fn label(hwnd: HWND) -> Option<Vec<u16>> {
    let index = SendMessageW(hwnd, CB_GETCURSEL, None, None).0;
    text::at(hwnd, index as usize)
}
unsafe fn draw(hwnd: HWND, dc: HDC) -> bool {
    let saved = SaveDC(dc);
    if saved == 0 {
        return false;
    }
    let _saved = Saved(dc, saved);
    let mut rect = RECT::default();
    let mut info = COMBOBOXINFO {
        cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
        ..Default::default()
    };
    if GetClientRect(hwnd, &mut rect).is_err() || GetComboBoxInfo(hwnd, &mut info).is_err() {
        return false;
    }
    let Some(mut text) = label(hwnd) else {
        return false;
    };
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    let enabled = IsWindowEnabled(hwnd).as_bool();
    SetDCBrushColor(
        dc,
        if enabled && messages::hovered(hwnd) {
            colors::HOVER
        } else {
            colors::BACKGROUND
        },
    );
    FillRect(dc, &rect, brush);
    let focused = GetFocus() == hwnd;
    let ink = if enabled {
        color::ink(hwnd, GetDCBrushColor(dc), colors::TEXT)
    } else {
        colors::MUTED
    };
    SetTextColor(dc, ink);
    SetBkMode(dc, TRANSPARENT);
    let font = SendMessageW(hwnd, WM_GETFONT, None, None).0;
    if font != 0 {
        SelectObject(dc, HGDIOBJ(font as *mut _));
    }
    let dpi = GetDpiForWindow(hwnd).clamp(48, 768);
    let pad = ((3 * dpi + 48) / 96) as i32;
    let mut text_rect = info.rcItem;
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
    // 共享 DC pen/brush 随 SaveDC 恢复，禁止删除系统 stock 对象。
    SelectObject(dc, GetStockObject(DC_PEN));
    SelectObject(dc, GetStockObject(DC_BRUSH));
    SetDCPenColor(dc, ink);
    SetDCBrushColor(dc, ink);
    let x = (info.rcButton.left + info.rcButton.right) / 2;
    let y = (info.rcButton.top + info.rcButton.bottom) / 2;
    let half = ((4 * dpi + 48) / 96).max(2) as i32;
    let _ = Polygon(
        dc,
        &[
            POINT {
                x: x - half,
                y: y - half / 2,
            },
            POINT {
                x: x + half,
                y: y - half / 2,
            },
            POINT { x, y: y + half / 2 },
        ],
    );
    SetDCBrushColor(
        dc,
        if focused && enabled {
            colors::FOCUS
        } else {
            colors::LINE
        },
    );
    let thickness = if focused && enabled {
        (2 * dpi).div_ceil(96) as i32
    } else {
        1
    };
    for _ in 0..thickness {
        if rect.right <= rect.left || rect.bottom <= rect.top {
            break;
        }
        FrameRect(dc, &rect, brush);
        rect.left += 1;
        rect.top += 1;
        rect.right -= 1;
        rect.bottom -= 1;
    }
    true
}
pub(super) unsafe fn message(hwnd: HWND, message: u32, wp: WPARAM) -> Option<LRESULT> {
    if !matches!(message, WM_PAINT | WM_PRINTCLIENT | WM_ERASEBKGND) || !colors::enabled() {
        return None;
    }
    if message == WM_ERASEBKGND {
        return Some(LRESULT(1));
    }
    if message == WM_PRINTCLIENT {
        return (wp.0 != 0 && draw(hwnd, HDC(wp.0 as *mut _))).then_some(LRESULT(0));
    }
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    if dc.is_invalid() {
        return None;
    }
    // 直接绘制避免先出现白色系统背景；失败时在同一已取得 DC 上回退系统绘制。
    if !draw(hwnd, dc) {
        DefSubclassProc(
            hwnd,
            WM_PRINTCLIENT,
            WPARAM(dc.0 as usize),
            LPARAM((PRF_CLIENT | PRF_ERASEBKGND) as isize),
        );
    }
    let _ = EndPaint(hwnd, &ps);
    Some(LRESULT(0))
}
