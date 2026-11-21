//! 小钉图保留可读控件和二维码尺寸，视口限于原钉图，滚动不改变分享来源。
use super::*;
use windows::Win32::UI::Controls::SetScrollInfo;

pub(super) unsafe fn position(
    hwnd: HWND,
    bar: SCROLLBAR_CONSTANTS,
    content: i32,
    viewport: i32,
) -> i32 {
    let mut info = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_ALL,
        ..Default::default()
    };
    let _ = GetScrollInfo(hwnd, bar, &mut info);
    info.nMin = 0;
    info.nMax = content.saturating_sub(1);
    info.nPage = viewport.max(1) as u32;
    info.nPos = info.nPos.clamp(0, (content - viewport).max(0));
    SetScrollInfo(hwnd, bar, &info, true);
    info.nPos
}
pub(super) unsafe fn scroll(hwnd: HWND, m: &Model, message: u32, wp: WPARAM) {
    let bar = if message == WM_HSCROLL {
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
        return;
    }
    let next = if message == WM_MOUSEWHEEL {
        info.nPos - ((wp.0 >> 16) as u16 as i16 as i32 / 120) * 48
    } else {
        match (wp.0 & 0xffff) as i32 {
            0 => info.nPos - 24,
            1 => info.nPos + 24,
            2 => info.nPos - info.nPage as i32,
            3 => info.nPos + info.nPage as i32,
            4 | 5 => info.nTrackPos,
            6 => 0,
            7 => info.nMax,
            _ => return,
        }
    };
    info.fMask = SIF_POS;
    info.nPos = next.clamp(0, (info.nMax + 1 - info.nPage as i32).max(0));
    SetScrollInfo(hwnd, bar, &info, true);
    layout::arrange(hwnd, m, windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd));
}
