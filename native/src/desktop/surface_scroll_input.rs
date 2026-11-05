//! 普通Surface与Tile共用系统滚轮偏好及屏幕→客户区解码；不注册监听、不注入输入。
use windows::Win32::{Foundation::*, Graphics::Gdi::ScreenToClient, UI::WindowsAndMessaging::*};
pub(super) struct Wheel {
    pub pixel: [i32; 2],
    pub axis: usize,
    pub delta: f32,
    pub page: bool,
}
pub(super) unsafe fn wheel(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> Option<Wheel> {
    if !matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) || wp.0 as u16 & 8 != 0 {
        return None;
    }
    let horizontal = message == WM_MOUSEHWHEEL || wp.0 as u16 & 4 != 0;
    let mut count = 3u32;
    let action = if horizontal {
        SPI_GETWHEELSCROLLCHARS
    } else {
        SPI_GETWHEELSCROLLLINES
    };
    if SystemParametersInfoW(
        action,
        0,
        Some((&mut count as *mut u32).cast()),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
    )
    .is_err()
    {
        count = 3;
    }
    let mut point = POINT {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 as usize >> 16) as u16 as i16 as i32,
    };
    if !ScreenToClient(hwnd, &mut point).as_bool() {
        return None;
    }
    let raw = (wp.0 >> 16) as u16 as i16 as f32;
    let sign = if message == WM_MOUSEHWHEEL { 1. } else { -1. };
    let page = count == u32::MAX;
    Some(Wheel {
        pixel: [point.x, point.y],
        axis: usize::from(!horizontal),
        delta: sign * raw / 120. * if page { 1. } else { 16. * count as f32 },
        page,
    })
}
