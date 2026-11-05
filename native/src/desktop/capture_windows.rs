//! 按桌面 Z 序缓存有限窗口边界；选区悬停不反复枚举窗口，不读取标题或进程数据。
use windows::{
    core::Result,
    Win32::{Foundation::*, Graphics::Dwm::*, UI::WindowsAndMessaging::*},
};

const LIMIT: usize = 2048;
pub(super) fn snapshot() -> Result<Vec<RECT>> {
    Ok(entries()?.into_iter().map(|(_, rect)| rect).collect())
}
/// 保留选择瞬间的 HWND；Live 在冻结前绑定进程身份，普通截图仍只消费矩形。
pub(super) fn entries() -> Result<Vec<(HWND, RECT)>> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(enumerate),
            LPARAM(&mut windows as *mut Vec<(HWND, RECT)> as isize),
        )?;
    }
    Ok(windows)
}
unsafe extern "system" fn enumerate(hwnd: HWND, parameter: LPARAM) -> windows::core::BOOL {
    let windows = &mut *(parameter.0 as *mut Vec<(HWND, RECT)>);
    // 返回 true 继续枚举但不再分配，避免把达到上限误报成 EnumWindows 错误。
    if windows.len() >= LIMIT || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return true.into();
    }
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        (&mut cloaked as *mut u32).cast(),
        std::mem::size_of::<u32>() as u32,
    )
    .is_ok()
        && cloaked != 0
    {
        return true.into();
    }
    let mut rect = RECT::default();
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        (&mut rect as *mut RECT).cast(),
        std::mem::size_of::<RECT>() as u32,
    )
    .is_err()
        && GetWindowRect(hwnd, &mut rect).is_err()
    {
        return true.into();
    }
    if rect.right > rect.left && rect.bottom > rect.top {
        windows.push((hwnd, rect));
    }
    true.into()
}

pub(super) fn local_rectangles(
    windows: &[RECT],
    origin: POINT,
    width: u32,
    height: u32,
) -> Vec<RECT> {
    windows
        .iter()
        .take(LIMIT)
        .filter_map(|rect| {
            let x = |v: i32| (v as i64 - origin.x as i64).clamp(0, width as i64) as i32;
            let y = |v: i32| (v as i64 - origin.y as i64).clamp(0, height as i64) as i32;
            let rect = RECT {
                left: x(rect.left),
                top: y(rect.top),
                right: x(rect.right),
                bottom: y(rect.bottom),
            };
            (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
        })
        .collect()
}
pub(super) fn pick(windows: &[RECT], point: POINT) -> Option<RECT> {
    windows
        .iter()
        .copied()
        .find(|r| point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_desktop_origin_clips_windows_and_preserves_z_order() {
        let windows = [
            RECT {
                left: -80,
                top: 10,
                right: 20,
                bottom: 90,
            },
            RECT {
                left: -200,
                top: -200,
                right: 200,
                bottom: 200,
            },
            RECT {
                left: 500,
                top: 0,
                right: 600,
                bottom: 20,
            },
        ];
        let local = local_rectangles(&windows, POINT { x: -100, y: 0 }, 200, 100);
        assert_eq!(local.len(), 2);
        let top = pick(&local, POINT { x: 30, y: 30 }).unwrap();
        assert_eq!(
            (top.left, top.top, top.right, top.bottom),
            (20, 10, 120, 90)
        );
        assert_eq!(pick(&local, POINT { x: 0, y: 0 }).unwrap().right, 200);
        assert!(pick(&local, POINT { x: 200, y: 100 }).is_none());
    }
    #[test]
    fn hostile_coordinates_and_count_are_bounded() {
        let rect = RECT {
            left: i32::MIN,
            top: i32::MIN,
            right: i32::MAX,
            bottom: i32::MAX,
        };
        let local = local_rectangles(
            &vec![rect; LIMIT + 10],
            POINT {
                x: i32::MIN,
                y: i32::MIN,
            },
            100,
            100,
        );
        assert_eq!(local.len(), LIMIT);
        assert_eq!(local[0].right, 100);
    }
}
