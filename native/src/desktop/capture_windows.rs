//! Cache bounded window bounds in desktop Z order; selection hover does not repeatedly enumerate windows or read titles/process data.
use windows::{
    core::Result,
    Win32::{Foundation::*, Graphics::Dwm::*, UI::WindowsAndMessaging::*},
};

const LIMIT: usize = 2048;
pub(super) fn snapshot() -> Result<Vec<RECT>> {
    collect(|_, rect| rect)
}
/// Live uses the same admission rules, then binds source identity from HWND.
pub(super) fn handles() -> Result<Vec<HWND>> {
    collect(|hwnd, _| hwnd)
}
fn collect<T, F: FnMut(HWND, RECT) -> T>(project: F) -> Result<Vec<T>> {
    let mut windows = Vec::new();
    let mut state = (&mut windows, project);
    unsafe {
        // EnumWindows callbacks are synchronous; state remains valid until every callback returns.
        EnumWindows(
            Some(enumerate::<T, F>),
            LPARAM(&mut state as *mut (&mut Vec<T>, F) as isize),
        )?;
    }
    Ok(windows)
}
unsafe extern "system" fn enumerate<T, F: FnMut(HWND, RECT) -> T>(
    hwnd: HWND,
    parameter: LPARAM,
) -> windows::core::BOOL {
    let (windows, project) = &mut *(parameter.0 as *mut (&mut Vec<T>, F));
    // Stop querying windows at the limit, but do not treat normal truncation as enumeration failure.
    if windows.len() < LIMIT {
        if let Some(rect) = eligible_bounds(hwnd) {
            windows.push(project(hwnd, rect));
        }
    }
    true.into()
}
unsafe fn eligible_bounds(hwnd: HWND) -> Option<RECT> {
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return None;
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
        return None;
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
        return None;
    }
    (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
}

pub(super) fn local_rectangles(
    mut windows: Vec<RECT>,
    origin: POINT,
    width: u32,
    height: u32,
) -> Vec<RECT> {
    windows.truncate(LIMIT);
    let x = |v: i32| (v as i64 - origin.x as i64).clamp(0, width as i64) as i32;
    let y = |v: i32| (v as i64 - origin.y as i64).clamp(0, height as i64) as i32;
    windows.retain_mut(|rect| {
        *rect = RECT {
            left: x(rect.left),
            top: y(rect.top),
            right: x(rect.right),
            bottom: y(rect.bottom),
        };
        rect.right > rect.left && rect.bottom > rect.top
    });
    windows
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
        let windows = vec![
            RECT {
                left: 500,
                top: 0,
                right: 600,
                bottom: 20,
            },
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
        ];
        let allocation = windows.as_ptr();
        let local = local_rectangles(windows, POINT { x: -100, y: 0 }, 200, 100);
        assert_eq!(local.as_ptr(), allocation);
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
        let mut windows = vec![rect; LIMIT + 10];
        windows[0] = RECT::default();
        let local = local_rectangles(
            windows,
            POINT {
                x: i32::MIN,
                y: i32::MIN,
            },
            100,
            100,
        );
        assert_eq!(local.len(), LIMIT - 1);
        assert_eq!(local[0].right, 100);
    }
}
