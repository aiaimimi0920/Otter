//! 主干 250×300 搜索菜单的原生布局与绘制；目录、授权和创建事务仍由 picker 拥有。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::Controls::*};
const SURFACE: u32 = 0x120d0a;

pub(super) unsafe fn visible(hwnd: HWND, id: i32, shown: bool) {
    let child = GetDlgItem(Some(hwnd), id).unwrap_or_default();
    if (GetWindowLongW(child, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0) != shown {
        let _ = ShowWindow(child, if shown { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
}
pub(super) unsafe fn position(hwnd: HWND, owner: HWND) -> Result<()> {
    let mut anchor = RECT::default();
    GetWindowRect(owner, &mut anchor)?;
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    GetMonitorInfoW(
        MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
        &mut monitor,
    )
    .ok()?;
    let dpi = GetDpiForWindow(owner).max(96) as i32;
    let width = (250 * dpi / 96).min(monitor.rcWork.right - monitor.rcWork.left);
    let height = (300 * dpi / 96).min(monitor.rcWork.bottom - monitor.rcWork.top);
    let x = ((i64::from(anchor.left) + i64::from(anchor.right) - i64::from(width)) / 2) as i32;
    let y = ((i64::from(anchor.top) + i64::from(anchor.bottom) - i64::from(height)) / 2) as i32;
    SetWindowPos(
        hwnd,
        None,
        x.clamp(monitor.rcWork.left, monitor.rcWork.right - width),
        y.clamp(monitor.rcWork.top, monitor.rcWork.bottom - height),
        width,
        height,
        SWP_NOACTIVATE | SWP_NOZORDER,
    )?;
    layout(hwnd)
}
pub(super) unsafe fn layout(hwnd: HWND) -> Result<()> {
    let dpi = GetDpiForWindow(hwnd).max(96);
    desktop::tool_panel::font::update(hwnd, dpi)?;
    let font = desktop::tool_panel::font::object(hwnd);
    let px = |n: i32| n * dpi as i32 / 96;
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect)?;
    let width = rect.right;
    let height = rect.bottom;
    for (id, x, y, w, h) in [
        (SEARCH, px(8), px(8), (width - px(76)).max(1), px(22)),
        (CLEAR, width - px(62), px(6), px(24), px(24)),
        (IDCANCEL.0, width - px(32), px(4), px(28), px(28)),
        (
            LIST,
            px(4),
            px(40),
            (width - px(8)).max(1),
            (height - px(44)).max(1),
        ),
        (STATUS, px(8), px(58), (width - px(16)).max(1), px(32)),
        (REFRESH, px(40), px(96), (width - px(80)).max(1), px(28)),
    ] {
        let control = GetDlgItem(Some(hwnd), id)?;
        SendMessageW(
            control,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(0)),
        );
        SetWindowPos(
            control,
            None,
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOREDRAW,
        )?;
    }
    SendMessageW(
        GetDlgItem(Some(hwnd), LIST)?,
        LB_SETITEMHEIGHT,
        Some(WPARAM(0)),
        Some(LPARAM(px(30) as isize)),
    );
    let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, px(18), px(18));
    if region.is_invalid() {
        return Err(Error::from_win32());
    }
    if SetWindowRgn(hwnd, Some(region), false) == 0 {
        let error = Error::from_win32();
        let _ = DeleteObject(HGDIOBJ(region.0));
        return Err(error);
    }
    Ok(())
}
pub(super) unsafe fn paint(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    ui: &Ui<'_>,
) -> Option<isize> {
    if msg == WM_ERASEBKGND {
        return Some(1);
    }
    if msg == WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        let result = neuro_mini_ui::paint_batch::window(hwnd, dc, |out| {
            let mut rect = RECT::default();
            GetClientRect(hwnd, &mut rect)?;
            theme::fill(out, &rect, SURFACE);
            Ok(())
        });
        let _ = EndPaint(hwnd, &ps);
        if let Err(error) = result {
            *ui.error.borrow_mut() = Some(error);
            let _ = EndDialog(hwnd, 0);
        }
        return Some(1);
    }
    if msg == WM_DRAWITEM && lp.0 != 0 {
        let item = &*(lp.0 as *const DRAWITEMSTRUCT);
        let result = neuro_mini_ui::paint_batch::control(item.hDC, item.rcItem, |dc| {
            let mut item = *item;
            item.hDC = dc;
            draw(&item, ui);
            Ok(())
        });
        if let Err(error) = result {
            *ui.error.borrow_mut() = Some(error);
            let _ = EndDialog(hwnd, 0);
        }
        return Some(1);
    }
    if matches!(msg, WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORLISTBOX) {
        let dc = HDC(wp.0 as *mut _);
        SetTextColor(dc, COLORREF(0x9f9a92));
        SetBkColor(dc, COLORREF(SURFACE));
        SetDCBrushColor(dc, COLORREF(SURFACE));
        return Some(GetStockObject(DC_BRUSH).0 as isize);
    }
    None
}
unsafe fn draw(item: &DRAWITEMSTRUCT, ui: &Ui<'_>) {
    let list = item.CtlID == LIST as u32;
    theme::fill(item.hDC, &item.rcItem, SURFACE);
    if list && item.itemID == u32::MAX {
        return;
    }
    let allowed = !list || (ui.context.allowed)(item.itemData);
    let selected = item.itemState.0 & (ODS_SELECTED.0 | ODS_FOCUS.0) != 0;
    let mut rect = item.rcItem;
    if list {
        rect.bottom -= (2 * GetDpiForWindow(item.hwndItem).max(96) / 96) as i32;
    }
    theme::fill(
        item.hDC,
        &rect,
        if !allowed {
            0x191410
        } else if selected {
            0x433930
        } else {
            0x2d2620
        },
    );
    SetTextColor(
        item.hDC,
        COLORREF(if allowed { 0xebe6e2 } else { 0x6b6159 }),
    );
    SetBkMode(item.hDC, TRANSPARENT);
    let font = SendMessageW(item.hwndItem, WM_GETFONT, None, None);
    let old = SelectObject(item.hDC, HGDIOBJ(font.0 as *mut _));
    let label = if list {
        ui.context
            .catalog
            .as_ref()
            .and_then(|c| c.handshake.capability(item.itemData))
            .map(|art| {
                format!(
                    "{}{}",
                    art.label,
                    if !art.enabled {
                        " · 已禁用"
                    } else if !allowed && ui.context.origin.is_none() {
                        " · 需选择 Loom 服务"
                    } else if !allowed {
                        " · 不可用"
                    } else {
                        ""
                    }
                )
            })
            .unwrap_or_default()
    } else if item.CtlID == CLEAR as u32 || item.CtlID == IDCANCEL.0 as u32 {
        "×".into()
    } else {
        text(item.hwndItem)
    };
    rect.left += 6;
    rect.right -= 6;
    DrawTextW(
        item.hDC,
        &mut label.encode_utf16().collect::<Vec<_>>(),
        &mut rect,
        DT_VCENTER
            | DT_SINGLELINE
            | DT_NOPREFIX
            | DT_END_ELLIPSIS
            | if list { DT_LEFT } else { DT_CENTER },
    );
    if item.itemState.0 & ODS_FOCUS.0 != 0 {
        SetDCBrushColor(item.hDC, COLORREF(0x38ffd9));
        FrameRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    }
    SelectObject(item.hDC, old);
}
