//! 固定内容宽度与内部滚动，物理工作区约束只读取不修改显示配置。
use super::*;
pub(super) unsafe fn initial(host: HWND) -> Result<RECT> {
    let mut anchor = RECT::default();
    GetWindowRect(host, &mut anchor)?;
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    GetMonitorInfoW(
        MonitorFromWindow(host, MONITOR_DEFAULTTONEAREST),
        &mut monitor,
    )
    .ok()?;
    let work = monitor.rcWork;
    let dpi = GetDpiForWindow(host).max(96);
    let width = (460 * dpi as i64 / 96)
        .min((work.right - work.left) as i64)
        .max(1) as i32;
    let height = (620 * dpi as i64 / 96)
        .min((work.bottom - work.top) as i64)
        .max(1) as i32;
    let x = anchor
        .right
        .saturating_add(8)
        .clamp(work.left, work.right - width);
    let y = anchor.top.clamp(work.top, work.bottom - height);
    Ok(RECT {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    })
}
pub(super) fn scaled(rect: RECT, dpi: u32) -> RECT {
    let scale = |value: i32| (i64::from(value) * i64::from(dpi.max(96)) / 96) as i32;
    RECT {
        left: scale(rect.left),
        top: scale(rect.top),
        right: scale(rect.right),
        bottom: scale(rect.bottom),
    }
}
pub(super) fn field_rect(ui: &Ui, rect: RECT) -> RECT {
    if ui.field.is_none() {
        return rect;
    }
    RECT {
        left: rect.left * 300 / 440,
        right: rect.right * 300 / 440,
        ..rect
    }
}
pub(super) unsafe fn place(ui: &Ui) {
    let hwnd = ui.hwnd.get();
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let mut client = RECT::default();
    if GetClientRect(hwnd, &mut client).is_err() {
        return;
    }
    let dpi = GetDpiForWindow(hwnd).max(96);
    let content = scaled(
        RECT {
            bottom: ui.content_height.get(),
            ..Default::default()
        },
        dpi,
    )
    .bottom;
    let scroll = ui.scroll.get().clamp(0, (content - client.bottom).max(0));
    ui.scroll.set(scroll);
    let width = scaled(
        RECT {
            right: if ui.field.is_some() { 300 } else { 440 },
            ..Default::default()
        },
        dpi,
    )
    .right;
    let horizontal = ui.scroll_x.get().clamp(0, (width - client.right).max(0));
    ui.scroll_x.set(horizontal);
    let info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: content.saturating_sub(1),
        nPage: client.bottom.max(1) as u32,
        nPos: scroll,
        ..Default::default()
    };
    SetScrollInfo(hwnd, SB_VERT, &info, true);
    SetScrollInfo(
        hwnd,
        SB_HORZ,
        &SCROLLINFO {
            nMax: width - 1,
            nPage: client.right.max(1) as u32,
            nPos: horizontal,
            ..info
        },
        true,
    );
    let placements = ui.placements.borrow().clone();
    for (child, rect) in placements {
        if !IsWindow(Some(child)).as_bool() {
            continue;
        }
        let rect = scaled(rect, dpi);
        let _ = SetWindowPos(
            child,
            None,
            rect.left - horizontal,
            rect.top - scroll,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}
pub(super) unsafe fn reveal(ui: &Ui, child: HWND) {
    let before = (ui.scroll.get(), ui.scroll_x.get());
    let rect = ui
        .placements
        .borrow()
        .iter()
        .find(|(hwnd, _)| *hwnd == child)
        .map(|(_, rect)| *rect);
    let Some(rect) = rect else {
        return;
    };
    let rect = scaled(rect, GetDpiForWindow(ui.hwnd.get()));
    let mut client = RECT::default();
    let _ = GetClientRect(ui.hwnd.get(), &mut client);
    let top = ui.scroll.get();
    let bottom = top + client.bottom;
    let next = if rect.top < top {
        rect.top
    } else if rect.bottom > bottom {
        rect.bottom - client.bottom
    } else {
        top
    };
    ui.scroll.set(next);
    let left = ui.scroll_x.get();
    if rect.left < left {
        ui.scroll_x.set(rect.left);
    } else if rect.right > left + client.right {
        ui.scroll_x.set(rect.right - client.right);
    }
    // IsDialogMessage也可同步处理paint；无滚动变化时重排会自续父paint并饿死child。
    if before != (ui.scroll.get(), ui.scroll_x.get()) {
        place(ui);
    }
}
pub(super) unsafe fn scroll(ui: &Ui, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    let horizontal = msg == WM_HSCROLL
        || msg == WM_MOUSEHWHEEL
        || msg == WM_MOUSEWHEEL && GetKeyState(VK_SHIFT.0 as i32) < 0;
    let position = if horizontal { &ui.scroll_x } else { &ui.scroll };
    if matches!(msg, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
        if GetKeyState(VK_CONTROL.0 as i32) < 0 {
            return false;
        }
        let delta = (wp.0 >> 16) as u16 as i16 as i32;
        let delta = delta / WHEEL_DELTA as i32 * 48;
        position.set(position.get().saturating_add(if msg == WM_MOUSEHWHEEL {
            delta
        } else {
            -delta
        }));
    } else if matches!(msg, WM_VSCROLL | WM_HSCROLL) && lp.0 == 0 {
        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..Default::default()
        };
        let _ = GetScrollInfo(
            ui.hwnd.get(),
            if horizontal { SB_HORZ } else { SB_VERT },
            &mut info,
        );
        let current = position.get();
        position.set(match SCROLLBAR_COMMAND((wp.0 & 0xffff) as i32) {
            SB_LINEUP => current - 28,
            SB_LINEDOWN => current + 28,
            SB_PAGEUP => current - info.nPage as i32,
            SB_PAGEDOWN => current + info.nPage as i32,
            SB_THUMBTRACK | SB_THUMBPOSITION => info.nTrackPos,
            SB_TOP => 0,
            SB_BOTTOM => info.nMax,
            _ => current,
        });
    } else {
        return false;
    }
    place(ui);
    true
}
