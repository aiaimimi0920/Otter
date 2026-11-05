//! 隐藏 HWND 的真实拖动路由与捕获；线程键盘快照不注入用户桌面输入。
use super::*;
use crate::{desktop::Apartment, live_frames::Source};
unsafe fn window() -> (Source<Texture>, LiveWindow) {
    let source = Source::new();
    let window = LiveWindow::new_at(
        source.subscribe().unwrap(),
        RECT {
            left: -500,
            top: -500,
            right: -244,
            bottom: -244,
        },
    )
    .unwrap();
    super::super::super::tests::publish(&source);
    SendMessageW(window.hwnd, WM_PAINT, None, None);
    (source, window)
}
unsafe fn mouse(window: &LiveWindow, message: u32, point: (i32, i32)) {
    let mut client = POINT {
        x: point.0,
        y: point.1,
    };
    assert!(ScreenToClient(window.hwnd, &mut client).as_bool());
    let lp = LPARAM(((client.x as u16 as u32) | ((client.y as u16 as u32) << 16)) as isize);
    SendMessageW(window.hwnd, message, None, Some(lp));
}
unsafe fn begin(window: &LiveWindow, resize: bool) {
    let keys = if resize {
        vec![VK_MENU, VK_SHIFT]
    } else {
        vec![VK_MENU]
    };
    super::super::tests::with_keys(&keys, || mouse(window, WM_LBUTTONDOWN, (-436, -436)));
    assert!(window.state.drag.get().is_some());
    assert_eq!(GetCapture(), window.hwnd);
}
#[test]
fn live_drag_begin_reentry_cannot_replace_pending_owner_and_cancel_stays_cancelled() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for cancelled in [false, true] {
            let (_source, window) = window();
            let lp = LPARAM(64 | (64 << 16));
            begin_with(window.hwnd, &window.state, lp, false, || {
                super::super::tests::with_keys(&[VK_MENU], || {
                    mouse(&window, WM_LBUTTONDOWN, (-400, -400))
                });
                assert!(window.state.drag.get().is_none());
                input::cancel(window.hwnd, &window.state);
                if cancelled {
                    SendMessageW(window.hwnd, WM_CANCELMODE, None, None);
                }
            });
            assert_eq!(window.state.drag.get().is_none(), cancelled);
            assert!(!window.state.drag_starting.get());
            if !cancelled {
                assert_eq!(window.state.drag.get().unwrap().pointer, (-436, -436));
                cancel(window.hwnd, &window.state);
            }
        }
    }
}
#[test]
fn live_drag_superseded_placement_never_retries_or_rolls_back_newer_geometry() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_source, window) = window();
        begin(&window, false);
        let before = client_rect(window.hwnd).unwrap();
        let newer = PinRect {
            x: -200,
            y: -100,
            ..before
        };
        let checks = Cell::new(0);
        let result = place_while(
            window.hwnd,
            PinRect {
                x: -400,
                y: -400,
                ..before
            },
            || {
                checks.set(checks.get() + 1);
                if checks.get() == 2 {
                    // 注入 SetWindowPos 返回后的同步重入：新位置提交且旧手势取消。
                    cancel(window.hwnd, &window.state);
                    place(window.hwnd, newer).unwrap();
                }
                window.state.drag.get().is_some()
            },
        );
        assert!(result.is_err());
        assert_eq!(client_rect(window.hwnd).unwrap(), newer);
        assert!(window.is_alive());
        assert_ne!(GetCapture(), window.hwnd);
    }
}
#[test]
fn live_drag_moves_from_screen_origin_and_keeps_local_capture_during_source_status() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_source, window) = window();
        window.input_status(true, false, None);
        mouse(&window, WM_LBUTTONDOWN, (-436, -436));
        assert_eq!(GetCapture(), window.hwnd);
        begin(&window, false);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        // Session 每轮刷新源控制状态；不得释放本地 owner 的捕获。
        window.input_status(false, false, None);
        assert_eq!(GetCapture(), window.hwnd);
        mouse(&window, WM_RBUTTONUP, (-300, -300));
        assert!(window.state.drag.get().is_some());
        assert_eq!(
            (
                client_rect(window.hwnd).unwrap().x,
                client_rect(window.hwnd).unwrap().y
            ),
            (-500, -500)
        );
        mouse(&window, WM_MOUSEMOVE, (-396, -416));
        let moved = client_rect(window.hwnd).unwrap();
        assert_eq!(
            (moved.x, moved.y, moved.width, moved.height),
            (-460, -480, 256, 256)
        );
        mouse(&window, WM_MOUSEMOVE, (-376, -406));
        assert_eq!(
            (
                client_rect(window.hwnd).unwrap().x,
                client_rect(window.hwnd).unwrap().y
            ),
            (-440, -470)
        );
        mouse(&window, WM_LBUTTONUP, (-366, -401));
        assert_eq!(
            (
                client_rect(window.hwnd).unwrap().x,
                client_rect(window.hwnd).unwrap().y
            ),
            (-430, -465)
        );
        assert!(window.state.drag.get().is_none());
        assert_ne!(GetCapture(), window.hwnd);
        let final_rect = client_rect(window.hwnd).unwrap();
        mouse(&window, WM_MOUSEMOVE, (-300, -300));
        assert_eq!(client_rect(window.hwnd).unwrap(), final_rect);
        assert_eq!(window.opacity(), 1.0);
        assert!(window.pop_input().is_none());
        let frame = window.frame_for_test().unwrap();
        assert_eq!((frame.resource.width, frame.resource.height), (2, 2));
    }
}
#[test]
fn live_drag_resize_uses_initial_client_ratio_and_exact_modifiers() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_source, window) = window();
        let original = client_rect(window.hwnd).unwrap();
        for keys in [
            &[][..],
            &[VK_SHIFT][..],
            &[VK_MENU, VK_CONTROL][..],
            &[VK_MENU, VK_LWIN][..],
        ] {
            super::super::tests::with_keys(keys, || mouse(&window, WM_LBUTTONDOWN, (-436, -436)));
            assert!(window.state.drag.get().is_none());
            assert_eq!(client_rect(window.hwnd).unwrap(), original);
        }
        begin(&window, true);
        mouse(&window, WM_MOUSEMOVE, (-372, -436));
        let next = client_rect(window.hwnd).unwrap();
        assert_eq!(
            next,
            crate::pin_geometry::resize_drag(original, (64, 0)).unwrap()
        );
        mouse(&window, WM_MOUSEMOVE, (-436, -436));
        assert_eq!(client_rect(window.hwnd).unwrap(), original);
        mouse(&window, WM_LBUTTONUP, (-436, -436));
        assert!(window.state.drag.get().is_none());
    }
}
#[test]
fn live_drag_lifecycle_cancels_without_stale_moves_or_foreign_capture_release() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for (message, wp) in [
            (WM_KILLFOCUS, 0),
            (WM_CANCELMODE, 0),
            (WM_ENTERMENULOOP, 0),
            (WM_ACTIVATE, WA_INACTIVE as usize),
            (WM_SIZE, SIZE_MINIMIZED as usize),
            (WM_SHOWWINDOW, 0),
            (WM_KEYDOWN, VK_ESCAPE.0 as usize),
        ] {
            let (_source, window) = window();
            begin(&window, false);
            SendMessageW(window.hwnd, message, Some(WPARAM(wp)), None);
            assert!(window.state.drag.get().is_none());
            assert_ne!(GetCapture(), window.hwnd);
            let before = client_rect(window.hwnd).unwrap();
            mouse(&window, WM_MOUSEMOVE, (-300, -300));
            assert_eq!(client_rect(window.hwnd).unwrap(), before);
        }
        let (_source, window) = window();
        let (_other_source, other) = self::window();
        begin(&window, false);
        SetCapture(other.hwnd);
        assert!(window.state.drag.get().is_none());
        assert_eq!(GetCapture(), other.hwnd);
        cancel(window.hwnd, &window.state);
        assert_eq!(GetCapture(), other.hwnd);
        let _ = ReleaseCapture();
        begin(&window, false);
        DestroyWindow(window.hwnd).unwrap();
        assert!(window.state.drag.get().is_none());
        assert_ne!(GetCapture(), window.hwnd);
    }
}
#[test]
fn live_drag_group_lock_and_hide_cancel_active_owner_and_reject_new_drag() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for hidden in [false, true] {
            let (_source, window) = window();
            let mut workspace = crate::workspace::Workspace::with_initial(window.unit_id().clone());
            let id = crate::workspace_id::UnitId::parse("drag-group").unwrap();
            workspace
                .upsert_group(crate::workspace::Group {
                    id: id.clone(),
                    name: "拖动测试".into(),
                    hidden: false,
                    locked: false,
                })
                .unwrap();
            workspace
                .assign_group(&[window.unit_id().clone()], Some(&id))
                .unwrap();
            window.sync_groups(&workspace).unwrap();
            begin(&window, false);
            workspace
                .upsert_group(crate::workspace::Group {
                    id,
                    name: "拖动测试".into(),
                    hidden,
                    locked: !hidden,
                })
                .unwrap();
            window.sync_groups(&workspace).unwrap();
            assert!(window.state.drag.get().is_none());
            assert_ne!(GetCapture(), window.hwnd);
            let before = client_rect(window.hwnd).unwrap();
            super::super::tests::with_keys(&[VK_MENU], || {
                mouse(&window, WM_LBUTTONDOWN, (-436, -436))
            });
            mouse(&window, WM_MOUSEMOVE, (-300, -300));
            assert!(window.state.drag.get().is_none());
            assert_eq!(client_rect(window.hwnd).unwrap(), before);
        }
    }
}
