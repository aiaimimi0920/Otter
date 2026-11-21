//! 同步 motion/hover 的显示来源仍受精确事务及首次消息泵约束。
use super::*;
use crate::{native_dispatch as dispatch, native_popup::Lease};
use windows::Win32::UI::Controls::{WM_MOUSEHOVER, WM_MOUSELEAVE};

#[test]
fn only_move_and_hover_extend_the_scalar_allowlist() {
    for message in [WM_MOUSEMOVE, WM_MOUSEHOVER] {
        assert!(valid_packet(message, 0x7f, u32::MAX as isize));
        assert!(!valid_packet(message, 0x80, 0));
        assert!(!valid_packet(message, 0, -1));
    }
    for message in [WM_TIMER, WM_MOUSELEAVE, WM_MOUSEWHEEL, WM_SETCURSOR] {
        assert!(!valid_packet(message, 0, 0));
    }
}

#[test]
fn synchronous_hover_popup_gets_an_exact_lease_but_post_pump_show_does_not() {
    for message in [WM_MOUSEMOVE, WM_MOUSEHOVER] {
        for pump in [false, true] {
            let f = Fixture::new();
            let popup = Window::new(None);
            let _hooks = Hooks::new();
            POPUP.with(|slot| slot.set(popup.0));
            BUTTON_HANDLER.with(|handler| {
                handler.set(Some(if pump {
                    || unsafe {
                        dispatch::message_pump_entered();
                        let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
                    }
                } else {
                    || unsafe {
                        let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
                    }
                }))
            });
            put(f.owner.0, MESSAGE, message as usize);
            assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
            BUTTON_HANDLER.with(|handler| handler.set(None));
            assert_eq!(value(f.owner.0, FINISHED), 303);
            let lease = Lease::read(popup.0);
            assert_eq!(lease.is_some(), !pump);
            if let Some(lease) = lease {
                assert_eq!(
                    (lease.owner, lease.root, lease.request),
                    (f.owner.0, f.root.0, 303)
                );
                // 单次领取仍有效，重复调用不能重放窗口过程。
                assert!(!dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
                put(f.owner.0, EPOCH, 0);
                assert!(!lease.valid());
            }
        }
    }
}

#[test]
#[ignore = "interactive desktop; positions only owned no-activate source/menu HWNDs near the cursor"]
fn owned_cursor_menu_is_relocated_without_a_lease_or_owner_change() {
    use windows::Win32::Graphics::Gdi::*;
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).unwrap();
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        assert!(GetMonitorInfoW(
            MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST),
            &mut info
        )
        .as_bool());
        let center = (i64::from(info.rcWork.left) + i64::from(info.rcWork.right)) / 2;
        let source_x = if i64::from(cursor.x) < center {
            info.rcWork.right - 100
        } else {
            info.rcWork.left + 100
        };
        let source_y = info.rcWork.top + 100;
        SetWindowPos(
            f.root.0,
            None,
            source_x,
            source_y,
            64,
            64,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, f.root.0 .0 as isize);
        SetWindowPos(
            popup.0,
            None,
            cursor.x,
            cursor.y,
            64,
            64,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        POPUP.with(|slot| slot.set(popup.0));
        BUTTON_HANDLER.with(|handler| {
            handler.set(Some(|| {
                let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
            }))
        });
        put(f.owner.0, MESSAGE, WM_MOUSEMOVE as usize);
        put(f.owner.0, LP, 0);
        let foreground = GetForegroundWindow();
        assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
        BUTTON_HANDLER.with(|handler| handler.set(None));
        let mut actual = RECT::default();
        GetWindowRect(popup.0, &mut actual).unwrap();
        assert_eq!((actual.left, actual.top), (source_x, source_y));
        assert_eq!(
            (actual.right - actual.left, actual.bottom - actual.top),
            (64, 64)
        );
        assert_eq!(GetWindow(popup.0, GW_OWNER).unwrap(), f.root.0);
        assert!(Lease::read(popup.0).is_none());
        assert_eq!(GetForegroundWindow(), foreground);

        let _ = ShowWindow(popup.0, SW_HIDE);
        let unrelated = Window::new(None);
        SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, unrelated.0 .0 as isize);
        SetWindowPos(
            popup.0,
            None,
            cursor.x,
            cursor.y,
            64,
            64,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        f.publish(304);
        put(f.owner.0, MESSAGE, WM_MOUSEMOVE as usize);
        put(f.owner.0, LP, 0);
        BUTTON_HANDLER.with(|handler| {
            handler.set(Some(|| {
                let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
            }))
        });
        assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
        BUTTON_HANDLER.with(|handler| handler.set(None));
        GetWindowRect(popup.0, &mut actual).unwrap();
        assert_eq!(
            (actual.left, actual.top),
            (cursor.x, cursor.y),
            "same process is not popup ownership"
        );
        assert_eq!(GetWindow(popup.0, GW_OWNER).unwrap(), unrelated.0);
        assert!(Lease::read(popup.0).is_none());
        assert_eq!(GetForegroundWindow(), foreground);
    }
}
