//! 面板滚动范围由实际内容/客户区推导；偏移由 Win32 scrollbar 持有，无定时器。
use windows::Win32::UI::Controls::{SetScrollInfo, SetScrollPos};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
#[path = "tool_panel_scroll_smoke.rs"]
mod smoke;
pub(super) use smoke::run as smoke;
#[path = "tool_panel_wheel.rs"]
mod wheel;
use std::cell::Cell;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::ScreenToClient,
    UI::{Input::KeyboardAndMouse::GetFocus, WindowsAndMessaging::*},
};

pub(super) unsafe fn sync(hwnd: HWND) -> POINT {
    let mut client = RECT::default();
    let _ = GetClientRect(hwnd, &mut client);
    let (width, height) = super::geometry::client(
        crate::desktop::tool_commands::TOOLS.len(),
        crate::desktop::tool_panel_actions::ACTIONS.len(),
        GetDpiForWindow(hwnd),
    );
    for (bar, extent, page) in [
        (SB_HORZ, width, client.right),
        (SB_VERT, height, client.bottom),
    ] {
        let info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_RANGE | SIF_PAGE | SIF_POS | SIF_DISABLENOSCROLL,
            nMin: 0,
            nMax: extent - 1,
            nPage: page.max(1) as u32,
            nPos: GetScrollPos(hwnd, bar),
            ..Default::default()
        };
        SetScrollInfo(hwnd, bar, &info, true);
    }
    POINT {
        x: GetScrollPos(hwnd, SB_HORZ),
        y: GetScrollPos(hwnd, SB_VERT),
    }
}
fn bounded(value: i64, max: i32, page: u32) -> i32 {
    value.clamp(0, (max as i64 - page as i64 + 1).max(0)) as i32
}
unsafe fn move_to(hwnd: HWND, bar: SCROLLBAR_CONSTANTS, value: i64) -> bool {
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_ALL,
        ..Default::default()
    };
    if GetScrollInfo(hwnd, bar, &mut info).is_err() {
        return false;
    }
    let next = bounded(value, info.nMax, info.nPage);
    if next == info.nPos {
        return false;
    }
    SetScrollPos(hwnd, bar, next, true);
    true
}
pub(super) unsafe fn dispatch(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    remainder: &Cell<[i64; 2]>,
) -> bool {
    if matches!(msg, WM_SETTINGCHANGE | WM_DPICHANGED) {
        remainder.set([0; 2]);
    }
    if !matches!(
        msg,
        WM_VSCROLL | WM_HSCROLL | WM_MOUSEWHEEL | WM_MOUSEHWHEEL
    ) {
        return false;
    }
    let bar = if matches!(msg, WM_HSCROLL | WM_MOUSEHWHEEL) {
        SB_HORZ
    } else {
        SB_VERT
    };
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_ALL,
        ..Default::default()
    };
    if GetScrollInfo(hwnd, bar, &mut info).is_err() {
        return true;
    }
    let line = (35 * GetDpiForWindow(hwnd).max(96) as i64 / 96).max(1);
    let axis = usize::from(bar == SB_VERT);
    let mut fractions = remainder.get();
    let next = if matches!(msg, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
        let horizontal = msg == WM_MOUSEHWHEEL;
        let delta = (wp.0 >> 16) as i16 as i32 * if horizontal { 1 } else { -1 };
        // 面板横向为非文本按钮网格，以 8 DIP 作为字符滚动单位。
        let unit = if horizontal {
            (8 * GetDpiForWindow(hwnd).max(96) as i64 / 96).max(1)
        } else {
            line
        };
        (info.nPos as i64).saturating_add(wheel::distance(
            delta,
            unit,
            wheel::preference(horizontal),
            info.nPage,
            &mut fractions[axis],
        ))
    } else {
        fractions[axis] = 0;
        match wp.0 as u16 as i32 {
            value if value == SB_LINEUP.0 => info.nPos as i64 - line,
            value if value == SB_LINEDOWN.0 => info.nPos as i64 + line,
            value if value == SB_PAGEUP.0 => info.nPos as i64 - info.nPage as i64,
            value if value == SB_PAGEDOWN.0 => info.nPos as i64 + info.nPage as i64,
            value if value == SB_TOP.0 => 0,
            value if value == SB_BOTTOM.0 => info.nMax as i64,
            value if value == SB_THUMBTRACK.0 || value == SB_THUMBPOSITION.0 => {
                info.nTrackPos as i64
            }
            _ => info.nPos as i64,
        }
    };
    remainder.set(fractions);
    if move_to(hwnd, bar, next) {
        super::layout(hwnd);
    }
    true
}
pub(super) unsafe fn reveal_focus(hwnd: HWND) {
    reveal(hwnd, GetFocus());
}
pub(super) unsafe fn reveal(hwnd: HWND, control: HWND) {
    if !IsChild(hwnd, control).as_bool() {
        return;
    }
    let mut rect = RECT::default();
    let mut client = RECT::default();
    if GetWindowRect(control, &mut rect).is_err() || GetClientRect(hwnd, &mut client).is_err() {
        return;
    }
    let mut start = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut end = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    if !ScreenToClient(hwnd, &mut start).as_bool() || !ScreenToClient(hwnd, &mut end).as_bool() {
        return;
    }
    let mut changed = false;
    for (bar, first, last, page) in [
        (SB_HORZ, start.x, end.x, client.right),
        (SB_VERT, start.y, end.y, client.bottom),
    ] {
        let delta = if first < 0 {
            first
        } else {
            (last - page).max(0)
        };
        changed |= move_to(hwnd, bar, GetScrollPos(hwnd, bar) as i64 + delta as i64);
    }
    if changed {
        super::layout(hwnd);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_clamps_large_offsets_and_content_smaller_than_viewport() {
        assert_eq!(bounded(i64::MAX, 999, 100), 900);
        assert_eq!(bounded(-1, 999, 100), 0);
        assert_eq!(bounded(30, 99, 200), 0);
        assert_eq!(bounded(75, 999, 100), 75);
    }
}
