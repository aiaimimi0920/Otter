//! 修正跟随真实鼠标创建的源菜单；只改变本次已认证 SHOW 的位置，不改 owner/层级。
use crate::native_protocol::Request;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

#[derive(Clone, Copy)]
pub(crate) struct Anchor {
    cursor: POINT,
    source: POINT,
}
impl Anchor {
    pub(crate) fn read(request: Request) -> Option<Self> {
        // 与源窗口过程使用相同 DPI 上下文；不能混入宿主的物理屏幕坐标。
        let mut source = POINT {
            x: request.lp as u16 as i16 as i32,
            y: ((request.lp as u32) >> 16) as u16 as i16 as i32,
        };
        let mut cursor = POINT::default();
        unsafe {
            GetCursorPos(&mut cursor).ok()?;
            if !ClientToScreen(request.destination, &mut source).as_bool() {
                return None;
            }
        }
        Some(Self { cursor, source })
    }

    pub(crate) fn place(self, popup: HWND) -> windows::core::Result<()> {
        unsafe {
            // 普通 dialog 可能本来就应居中，不能对所有弹窗施加 A-B 偏移。
            if GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_NOACTIVATE.0 == 0 {
                return Ok(());
            }
            let mut rect = RECT::default();
            GetWindowRect(popup, &mut rect)?;
            let monitor = MonitorFromPoint(self.source, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return Err(windows::core::Error::from_win32());
            }
            if let Some(point) = relocated(rect, self.cursor, self.source, info.rcWork) {
                SetWindowPos(
                    popup,
                    None,
                    point.x,
                    point.y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                )?;
            }
        }
        Ok(())
    }
}

fn near(rect: RECT, point: POINT) -> bool {
    let x = i64::from(point.x);
    let y = i64::from(point.y);
    x >= i64::from(rect.left) - 8
        && x <= i64::from(rect.right) + 8
        && y >= i64::from(rect.top) - 8
        && y <= i64::from(rect.bottom) + 8
}

fn relocated(rect: RECT, cursor: POINT, source: POINT, work: RECT) -> Option<POINT> {
    if !near(rect, cursor) || near(rect, source) {
        return None;
    }
    let width = i64::from(rect.right) - i64::from(rect.left);
    let height = i64::from(rect.bottom) - i64::from(rect.top);
    if width <= 0 || height <= 0 || work.right <= work.left || work.bottom <= work.top {
        return None;
    }
    let x = i64::from(rect.left) + i64::from(source.x) - i64::from(cursor.x);
    let y = i64::from(rect.top) + i64::from(source.y) - i64::from(cursor.y);
    // 超过工作区大小的菜单保持尺寸，由应用自行滚动，不偷偷缩放客户区。
    Some(POINT {
        x: x.clamp(
            i64::from(work.left),
            (i64::from(work.right) - width).max(i64::from(work.left)),
        ) as i32,
        y: y.clamp(
            i64::from(work.top),
            (i64::from(work.bottom) - height).max(i64::from(work.top)),
        ) as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_window_moves_without_activation_owner_or_style_changes() {
        unsafe {
            let popup = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                windows::core::w!("STATIC"),
                windows::core::w!(""),
                WS_POPUP,
                100,
                100,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            struct Window(HWND);
            impl Drop for Window {
                fn drop(&mut self) {
                    unsafe {
                        let _ = DestroyWindow(self.0);
                    }
                }
            }
            let _window = Window(popup);
            let foreground = GetForegroundWindow();
            let style = GetWindowLongPtrW(popup, GWL_EXSTYLE);
            let source = POINT { x: 400, y: 400 };
            let monitor = MonitorFromPoint(source, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            assert!(GetMonitorInfoW(monitor, &mut info).as_bool());
            let cursor = POINT { x: 100, y: 100 };
            let expected =
                relocated(rect(100, 100, 100, 100), cursor, source, info.rcWork).unwrap();
            Anchor { cursor, source }.place(popup).unwrap();
            let mut actual = RECT::default();
            GetWindowRect(popup, &mut actual).unwrap();
            assert_eq!((actual.left, actual.top), (expected.x, expected.y));
            assert_eq!(
                (actual.right - actual.left, actual.bottom - actual.top),
                (100, 100)
            );
            assert_eq!(GetWindowLongPtrW(popup, GWL_EXSTYLE), style);
            assert_eq!(GetWindowLongPtrW(popup, GWLP_HWNDPARENT), 0);
            assert_eq!(GetForegroundWindow(), foreground);
            // 普通 dialog 不应沿用菜单的坐标修正。
            SetWindowLongPtrW(
                popup,
                GWL_EXSTYLE,
                (style as u32 & !WS_EX_NOACTIVATE.0) as isize,
            );
            Anchor {
                cursor: POINT {
                    x: actual.left,
                    y: actual.top,
                },
                source: POINT { x: 800, y: 800 },
            }
            .place(popup)
            .unwrap();
            let mut unchanged = RECT::default();
            GetWindowRect(popup, &mut unchanged).unwrap();
            assert_eq!(actual, unchanged);
        }
    }
    fn rect(x: i32, y: i32, width: i32, height: i32) -> RECT {
        RECT {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }
    #[test]
    fn cursor_menu_moves_to_source_not_twice_to_b() {
        let result = relocated(
            rect(903, 187, 185, 572),
            POINT { x: 903, y: 187 },
            POINT { x: 2095, y: 259 },
            rect(0, 0, 3840, 2160),
        )
        .unwrap();
        assert_eq!((result.x, result.y), (2095, 259));
    }
    #[test]
    fn already_source_anchored_and_unrelated_dialog_are_unchanged() {
        let menu = rect(900, 180, 200, 500);
        let work = rect(0, 0, 3840, 2160);
        assert!(relocated(
            menu,
            POINT { x: 903, y: 187 },
            POINT { x: 910, y: 200 },
            work
        )
        .is_none());
        assert!(relocated(
            menu,
            POINT { x: 100, y: 100 },
            POINT { x: 2095, y: 259 },
            work
        )
        .is_none());
    }
    #[test]
    fn negative_monitor_and_oversized_menu_keep_size_and_fit_origin() {
        let result = relocated(
            rect(100, 100, 200, 900),
            POINT { x: 100, y: 100 },
            POINT { x: -10, y: 700 },
            rect(-1920, 0, 1920, 800),
        )
        .unwrap();
        assert_eq!((result.x, result.y), (-200, 0));
    }
}
