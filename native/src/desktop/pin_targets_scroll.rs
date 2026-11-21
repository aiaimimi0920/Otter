//! 目标列表的窄自绘滚动条；原 LISTBOX 仍拥有行索引、键盘选择及滚轮。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
pub(super) const ID: i32 = 12;

unsafe fn list(bar: HWND) -> Option<HWND> {
    GetDlgItem(GetParent(bar).ok(), LIST).ok()
}
unsafe fn geometry(bar: HWND) -> Option<(HWND, RECT, RECT, i32)> {
    let list = list(bar)?;
    let mut bounds = RECT::default();
    GetClientRect(bar, &mut bounds).ok()?;
    let mut client = RECT::default();
    GetClientRect(list, &mut client).ok()?;
    let count = SendMessageW(list, LB_GETCOUNT, None, None).0 as i32;
    let row = SendMessageW(list, LB_GETITEMHEIGHT, Some(WPARAM(0)), None)
        .0
        .max(1) as i32;
    let page = (client.bottom / row).max(1);
    let maximum = (count - page).max(0);
    if maximum == 0 {
        return None;
    }
    let top = SendMessageW(list, LB_GETTOPINDEX, None, None).0 as i32;
    let size = (bounds.bottom * page / count).max(16).min(bounds.bottom);
    let y = (bounds.bottom - size) * top / maximum;
    let thumb = RECT {
        left: 1,
        top: y,
        right: (bounds.right - 1).max(2),
        bottom: y + size,
    };
    Some((list, bounds, thumb, maximum))
}
pub(super) unsafe fn install(bar: HWND) {
    let _ = SetWindowSubclass(bar, Some(procedure), 1, 0);
}
pub(super) unsafe fn changed(list: HWND) {
    if let Ok(bar) = GetDlgItem(GetParent(list).ok(), ID) {
        let _ = InvalidateRect(Some(bar), None, false);
    }
}
pub(super) unsafe fn paint(item: &DRAWITEMSTRUCT) {
    let p = neuro_mini_ui::Palette::default();
    SetDCBrushColor(item.hDC, color(p.surface));
    FillRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    if let Some((_, _, thumb, _)) = geometry(item.hwndItem) {
        SetDCBrushColor(
            item.hDC,
            color(if GetCapture() == item.hwndItem {
                p.signal
            } else {
                p.muted
            }),
        );
        FillRect(item.hDC, &thumb, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
}
unsafe fn scroll(bar: HWND, y: i32, drag: Option<i32>) {
    let Some((list, bounds, thumb, maximum)) = geometry(bar) else {
        return;
    };
    let top = if let Some(offset) = drag {
        let travel = bounds.bottom - (thumb.bottom - thumb.top);
        if travel == 0 {
            return;
        }
        ((y - offset).clamp(0, travel) * maximum + travel / 2) / travel
    } else {
        let current = SendMessageW(list, LB_GETTOPINDEX, None, None).0 as i32;
        let row = SendMessageW(list, LB_GETITEMHEIGHT, Some(WPARAM(0)), None)
            .0
            .max(1) as i32;
        (current
            + if y < thumb.top {
                -bounds.bottom / row
            } else {
                bounds.bottom / row
            })
        .clamp(0, maximum)
    };
    SendMessageW(list, LB_SETTOPINDEX, Some(WPARAM(top as usize)), None);
    changed(list);
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    let y = (lp.0 >> 16) as i16 as i32;
    match msg {
        WM_LBUTTONDOWN => {
            if let Some((_, _, thumb, _)) = geometry(hwnd) {
                if y >= thumb.top && y < thumb.bottom {
                    let _ =
                        SetWindowSubclass(hwnd, Some(procedure), id, (y - thumb.top + 1) as usize);
                    SetCapture(hwnd);
                    let _ = InvalidateRect(Some(hwnd), None, false);
                } else {
                    scroll(hwnd, y, None);
                }
            }
            return LRESULT(0);
        }
        WM_MOUSEMOVE if GetCapture() == hwnd && data > 0 => {
            scroll(hwnd, y, Some(data as i32 - 1));
            return LRESULT(0);
        }
        WM_LBUTTONUP | WM_CANCELMODE if GetCapture() == hwnd => {
            let _ = ReleaseCapture();
        }
        WM_CAPTURECHANGED => {
            let _ = SetWindowSubclass(hwnd, Some(procedure), id, 0);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_MOUSEWHEEL => {
            if let Some(list) = list(hwnd) {
                return SendMessageW(list, msg, Some(wp), Some(lp));
            }
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(procedure), id);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
