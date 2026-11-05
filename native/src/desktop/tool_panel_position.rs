//! 面板定位使用物理像素与显示器工作区；不假设主屏原点为零，也不轮询窗口位置。
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};

fn clamp(candidate: RECT, work: RECT) -> RECT {
    let width = (candidate.right as i64 - candidate.left as i64)
        .max(1)
        .min((work.right as i64 - work.left as i64).max(1));
    let height = (candidate.bottom as i64 - candidate.top as i64)
        .max(1)
        .min((work.bottom as i64 - work.top as i64).max(1));
    // 客户区不足时由滚动条提供剩余内容，窗口本身不超出工作区。
    let left = (candidate.left as i64).clamp(
        work.left as i64,
        (work.right as i64 - width).max(work.left as i64),
    );
    let top = (candidate.top as i64).clamp(
        work.top as i64,
        (work.bottom as i64 - height).max(work.top as i64),
    );
    RECT {
        left: left as i32,
        top: top as i32,
        right: (left + width).min(i32::MAX as i64) as i32,
        bottom: (top + height).min(i32::MAX as i64) as i32,
    }
}
fn place(anchor: RECT, work: RECT, width: i32, height: i32, gap: i32) -> RECT {
    let below = anchor.bottom as i64 + gap as i64;
    let above = anchor.top as i64 - gap as i64 - height as i64;
    let top = if below + height as i64 <= work.bottom as i64 {
        below
    } else if above >= work.top as i64 {
        above
    } else {
        work.top as i64
    };
    let candidate = RECT {
        left: anchor.left,
        top: top.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        right: (anchor.left as i64 + width as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        bottom: (top + height as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
    };
    clamp(candidate, work)
}
unsafe fn work_area(rect: &RECT) -> Result<RECT> {
    let monitor = MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return Err(Error::from_win32());
    }
    Ok(info.rcWork)
}
pub(super) unsafe fn initial(anchor: RECT, width: i32, height: i32, gap: i32) -> Result<RECT> {
    Ok(place(anchor, work_area(&anchor)?, width, height, gap))
}
pub(super) unsafe fn fit(hwnd: HWND, proposed: Option<RECT>) -> Result<()> {
    let rect = if let Some(rect) = proposed {
        rect
    } else {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;
        rect
    };
    let bounded = clamp(rect, work_area(&rect)?);
    SetWindowPos(
        hwnd,
        None,
        bounded.left,
        bounded.top,
        bounded.right - bounded.left,
        bounded.bottom - bounded.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
        RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }
    #[test]
    fn below_then_above_avoids_taskbar_and_preserves_size() {
        let work = rect(0, 0, 1920, 1040);
        assert_eq!(
            place(rect(200, 100, 600, 400), work, 400, 180, 8),
            rect(200, 508, 400, 180)
        );
        assert_eq!(
            place(rect(1800, 850, 120, 180), work, 400, 180, 8),
            rect(1520, 662, 400, 180)
        );
    }
    #[test]
    fn negative_monitor_origin_and_disconnected_screen_are_clamped() {
        let work = rect(-1920, -300, 1920, 1040);
        assert_eq!(
            place(rect(-1900, 650, 100, 80), work, 500, 200, 12),
            rect(-1900, 438, 500, 200)
        );
        assert_eq!(
            clamp(rect(3000, 2000, 500, 200), work),
            rect(-500, 540, 500, 200)
        );
        assert_eq!(
            clamp(rect(-3000, -2000, 500, 200), work),
            rect(-1920, -300, 500, 200)
        );
    }
    #[test]
    fn fitting_is_stable_and_oversized_panel_is_limited_to_work_area() {
        let work = rect(10, 20, 300, 150);
        assert_eq!(
            clamp(rect(-100, -100, 500, 200), work),
            rect(10, 20, 300, 150)
        );
        let within = rect(30, 40, 100, 80);
        assert_eq!(clamp(within, work), within);
        assert_eq!(
            place(rect(20, 20, 200, 150), work, 200, 130, 8),
            rect(20, 20, 200, 130)
        );
    }
}
