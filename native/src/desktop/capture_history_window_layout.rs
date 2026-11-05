//! 历史标准按钮的DPI布局与内部滚动；不改变物理显示，不分配图像。
use super::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
fn scaled(hwnd: HWND, value: i32) -> i32 {
    (i64::from(value) * unsafe { GetDpiForWindow(hwnd) }.max(96) as i64 / 96) as i32
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
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client)?;
    let width = client.right.max(scaled(hwnd, 240));
    let margin = scaled(hwnd, 12);
    let row = scaled(hwnd, 126);
    let shot_top = scaled(hwnd, 224);
    let (colors, shots) = {
        let data = state.data.borrow();
        (
            data.colors
                .iter()
                .map(|e| e.hex.clone())
                .collect::<Vec<_>>(),
            data.screenshots
                .iter()
                .enumerate()
                .map(|(i, e)| format!("截图 {} · {}×{}", i + 1, e.width, e.height))
                .collect::<Vec<_>>(),
        )
    };
    let content = shot_top + row * shots.len().div_ceil(3) as i32 + scaled(hwnd, 48);
    let max_scroll = (content - client.bottom).max(0);
    state.scroll.set(state.scroll.get().clamp(0, max_scroll));
    let scroll = state.scroll.get();
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
        let label = match base {
            0 => colors.get(index).cloned(),
            1 => shots.get(index).cloned(),
            _ => shots.get(index).map(|_| format!("删除截图 {}", index + 1)),
        };
        let Some(label) = label else {
            let _ = ShowWindow(child, SW_HIDE);
            continue;
        };
        SetWindowTextW(child, &HSTRING::from(label))?;
        let (x, y, w, h) = if base == 0 {
            let cell = (width - 2 * margin) / 6;
            (
                margin + index as i32 % 6 * cell,
                scaled(hwnd, 44) + index as i32 / 6 * scaled(hwnd, 34),
                cell - 4,
                scaled(hwnd, 30),
            )
        } else {
            let cell = (width - 2 * margin) / 3;
            (
                margin + index as i32 % 3 * cell,
                shot_top + index as i32 / 3 * row + if base == 2 { scaled(hwnd, 94) } else { 0 },
                cell - 6,
                scaled(hwnd, if base == 2 { 24 } else { 90 }),
            )
        };
        MoveWindow(child, x, y - scroll, w, h, true)?;
        let _ = ShowWindow(child, SW_SHOWNOACTIVATE);
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    Ok(())
}
pub(super) unsafe fn scroll(hwnd: HWND, state: &State, msg: u32, wp: WPARAM) {
    let next = if msg == WM_MOUSEWHEEL {
        state
            .scroll
            .get()
            .saturating_sub(((wp.0 >> 16) as u16 as i16 as i32 / 120) * scaled(hwnd, 68))
    } else {
        let mut info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..Default::default()
        };
        let _ = GetScrollInfo(hwnd, SB_VERT, &mut info);
        match wp.0 as u16 as i32 {
            0 => state.scroll.get() - scaled(hwnd, 34),
            1 => state.scroll.get() + scaled(hwnd, 34),
            2 => state.scroll.get() - info.nPage as i32,
            3 => state.scroll.get() + info.nPage as i32,
            4 | 5 => info.nTrackPos,
            6 => 0,
            7 => info.nMax,
            _ => state.scroll.get(),
        }
    };
    state.scroll.set(next.max(0));
    let _ = update(hwnd, state);
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
        state.scroll.set((state.scroll.get() + delta).max(0));
        let _ = update(hwnd, state);
    }
}
