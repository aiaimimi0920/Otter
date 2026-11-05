//! 实时 GDI 单像素取色与不激活的原生预览窗口；DC 每次采样后立即释放。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::HiDpi::GetDpiForWindow};
#[path = "screen_color_swatch.rs"]
mod swatch;
struct DesktopDc(HDC);
impl Drop for DesktopDc {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseDC(None, self.0);
        }
    }
}
pub(super) unsafe fn sample(point: POINT, popup: Option<HWND>) -> Option<Color> {
    if let Some(popup) = popup {
        let mut rect = RECT::default();
        if IsWindowVisible(popup).as_bool()
            && GetWindowRect(popup, &mut rect).is_ok()
            && point.x >= rect.left
            && point.x < rect.right
            && point.y >= rect.top
            && point.y < rect.bottom
        {
            // 鼠标跳进旧预览矩形时先隐藏并等合成刷新，不能把自己的预览文字当屏幕颜色。
            let _ = ShowWindow(popup, SW_HIDE);
            windows::Win32::Graphics::Dwm::DwmFlush().ok()?;
        }
    }
    let dc = DesktopDc(GetDC(None));
    if dc.0.is_invalid() {
        return None;
    }
    let value = GetPixel(dc.0, point.x, point.y);
    decode(value)
}
fn decode(value: COLORREF) -> Option<Color> {
    (value.0 != 0xffff_ffff).then_some(Color(
        value.0 as u8,
        (value.0 >> 8) as u8,
        (value.0 >> 16) as u8,
    ))
}
pub(super) unsafe fn create(owner: HWND) -> Result<crate::desktop::Window> {
    let window = crate::desktop::Window(CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
        w!("STATIC"),
        w!(""),
        WS_POPUP | WS_BORDER | WS_DISABLED,
        0,
        0,
        160,
        30,
        Some(owner),
        None,
        None,
        None,
    )?);
    swatch::attach(window.0)?;
    Ok(window)
}
pub(super) unsafe fn show(hwnd: HWND, point: POINT, color: Color) {
    let changed = swatch::update(hwnd, color);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return;
    }
    let rect = info.rcWork;
    let mut dpi = GetDpiForWindow(hwnd);
    // 跨屏移动可能在第一次定位时改变 DPI；最多再校正一次，不创建布局轮询。
    for _ in 0..2 {
        let (width, height) = swatch::size(dpi);
        let x = place(point.x, rect.left, rect.right, width);
        let y = place(point.y, rect.top, rect.bottom, height);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let next = GetDpiForWindow(hwnd);
        if next == dpi {
            break;
        }
        dpi = next;
    }
    if changed {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}
fn place(cursor: i32, low: i32, high: i32, size: i32) -> i32 {
    let after = cursor.saturating_add(16);
    let before = cursor.saturating_sub(size).saturating_sub(16);
    let desired = if after.saturating_add(size) > high {
        before
    } else {
        after
    };
    desired.clamp(low, high.saturating_sub(size).max(low))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_pixel_is_not_black_and_preview_stays_on_negative_coordinate_monitor() {
        assert_eq!(decode(COLORREF(0x00563412)), Some(Color(0x12, 0x34, 0x56)));
        assert_eq!(decode(COLORREF(0)), Some(Color(0, 0, 0)));
        assert_eq!(decode(COLORREF(0xffff_ffff)), None);
        assert_eq!(place(-1900, -1920, 0, 160), -1884);
        assert!(place(-2, -1920, 0, 160) <= -160);
        assert_eq!(place(i32::MAX, -10, 10, 160), -10);
    }
}
