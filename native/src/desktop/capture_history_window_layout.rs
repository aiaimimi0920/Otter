//! 历史标准按钮的DPI布局与内部滚动；不改变物理显示，不分配图像。
use super::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
fn scaled(dpi: u32, value: i32) -> i32 {
    (i64::from(value) * i64::from(dpi.max(96)) / 96) as i32
}
pub(super) unsafe fn create(hwnd: HWND, state: &State) -> Result<()> {
    for base in [100, 200, 300] {
        for index in 0..24 {
            let child = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                None,
                WS_CHILD | WS_TABSTOP | WINDOW_STYLE((BS_OWNERDRAW | BS_NOTIFY) as u32),
                0,
                0,
                0,
                0,
                Some(hwnd),
                Some(HMENU((base + index) as *mut _)),
                None,
                None,
            )?;
            SendMessageW(
                child,
                WM_SETFONT,
                Some(WPARAM(GetStockObject(DEFAULT_GUI_FONT).0 as usize)),
                None,
            );
            state.children.borrow_mut().push(child);
        }
    }
    Ok(())
}
pub(super) unsafe fn update(hwnd: HWND, state: &State) -> Result<()> {
    // HWND messages may reenter the parent. Keep an owned snapshot, not a live RefCell borrow.
    let data = state.data.borrow().clone();
    let children = state.children.borrow().clone();
    for (slot, child) in children.into_iter().enumerate() {
        let index = slot % 24;
        let label = match slot / 24 {
            0 => data
                .colors
                .get(index)
                .map(|entry| HSTRING::from(&entry.hex)),
            1 => data.screenshots.get(index).map(|entry| {
                HSTRING::from(format!(
                    "截图 {} · {}×{}",
                    index + 1,
                    entry.width,
                    entry.height
                ))
            }),
            _ => data
                .screenshots
                .get(index)
                .map(|_| HSTRING::from(format!("删除截图 {}", index + 1))),
        };
        if let Some(label) = label {
            SetWindowTextW(child, &label)?;
        }
    }
    geometry(hwnd, state, true, None)
}

pub(super) unsafe fn resize(hwnd: HWND, state: &State) -> Result<()> {
    geometry(hwnd, state, false, None)
}

unsafe fn geometry(
    hwnd: HWND,
    state: &State,
    refresh_visibility: bool,
    requested_scroll: Option<i32>,
) -> Result<()> {
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client)?;
    let dpi = GetDpiForWindow(hwnd);
    let scale = |value| scaled(dpi, value);
    let width = client.right.max(scale(240));
    let margin = scale(12);
    let row = scale(126);
    let shot_top = scale(224);
    let (colors, shots) = {
        let data = state.data.borrow();
        (data.colors.len(), data.screenshots.len())
    };
    let content = shot_top + row * shots.div_ceil(3) as i32 + scale(48);
    let max_scroll = (content - client.bottom).max(0);
    let current = state.scroll.get();
    let scroll = requested_scroll.unwrap_or(current).clamp(0, max_scroll);
    // At a scroll boundary the effective position has not changed. Resize and data
    // updates still run layout because the viewport or its contents may have changed.
    if requested_scroll.is_some() && scroll == current {
        return Ok(());
    }
    state.scroll.set(scroll);
    SetScrollInfo(
        hwnd,
        SB_VERT,
        &SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
            nMin: 0,
            nMax: content - 1,
            nPage: client.bottom.max(0) as u32,
            nPos: scroll,
            ..Default::default()
        },
        true,
    );
    let children = state.children.borrow().clone();
    for (slot, child) in children.into_iter().enumerate() {
        let index = slot % 24;
        let base = slot / 24;
        if index >= if base == 0 { colors } else { shots } {
            if refresh_visibility {
                let _ = ShowWindow(child, SW_HIDE);
            }
            continue;
        }
        let (x, y, w, h) = if base == 0 {
            let cell = (width - 2 * margin) / 6;
            (
                margin + index as i32 % 6 * cell,
                scale(44) + index as i32 / 6 * scale(34),
                cell - 4,
                scale(30),
            )
        } else {
            let cell = (width - 2 * margin) / 3;
            (
                margin + index as i32 % 3 * cell,
                shot_top + index as i32 / 3 * row + if base == 2 { scale(94) } else { 0 },
                cell - 6,
                scale(if base == 2 { 24 } else { 90 }),
            )
        };
        MoveWindow(child, x, y - scroll, w, h, true)?;
        if refresh_visibility {
            let _ = ShowWindow(child, SW_SHOWNOACTIVATE);
        }
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    Ok(())
}
pub(super) unsafe fn scroll(hwnd: HWND, state: &State, msg: u32, wp: WPARAM) {
    let next = if msg == WM_MOUSEWHEEL {
        state.scroll.get().saturating_sub(
            ((wp.0 >> 16) as u16 as i16 as i32 / 120) * scaled(GetDpiForWindow(hwnd), 68),
        )
    } else {
        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..Default::default()
        };
        let _ = GetScrollInfo(hwnd, SB_VERT, &mut info);
        match wp.0 as u16 as i32 {
            0 => state.scroll.get() - scaled(GetDpiForWindow(hwnd), 34),
            1 => state.scroll.get() + scaled(GetDpiForWindow(hwnd), 34),
            2 => state.scroll.get() - info.nPage as i32,
            3 => state.scroll.get() + info.nPage as i32,
            4 | 5 => info.nTrackPos,
            6 => 0,
            7 => info.nMax,
            _ => state.scroll.get(),
        }
    };
    if next != state.scroll.get() {
        let _ = geometry(hwnd, state, false, Some(next));
    }
}
pub(super) unsafe fn focus(hwnd: HWND, state: &State, child: HWND) {
    let mut rect = RECT::default();
    let mut client = RECT::default();
    if GetWindowRect(child, &mut rect).is_err() || GetClientRect(hwnd, &mut client).is_err() {
        return;
    }
    let mut point = POINT {
        x: rect.left,
        y: rect.top,
    };
    let _ = ScreenToClient(hwnd, &mut point);
    let bottom = point.y + rect.bottom - rect.top;
    let delta = if point.y < 0 {
        point.y
    } else {
        (bottom - client.bottom).max(0)
    };
    if delta != 0 {
        let next = state.scroll.get().saturating_add(delta);
        let _ = geometry(hwnd, state, false, Some(next));
    }
}
