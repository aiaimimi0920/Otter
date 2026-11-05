//! 隐藏自有历史窗口/标准按钮；不申请前台，不操作剪贴板或其他进程。
use super::*;
#[test]
fn owned_buttons_display_both_lists_queue_stable_actions_and_close_without_quitting_application() {
    unsafe {
        let _com = crate::desktop::Apartment::new().unwrap();
        let source = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("history fixture"),
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let shared: Shared = Default::default();
        shared
            .borrow_mut()
            .color(crate::stroke_style::Color(1, 2, 3));
        let image = crate::image_decode::Image {
            width: 480,
            height: 240,
            pixels: [0, 0, 255, 255].repeat(480 * 240).into(),
        };
        shared
            .borrow_mut()
            .data
            .screenshot(crate::capture_history::image::capture("id".into(), &image, 1).unwrap());
        let origin = Origin {
            unit: crate::workspace_id::UnitId::parse("fixture").unwrap(),
            lifetime: Default::default(),
        };
        let window = Window::new(source.0, origin, shared.clone()).unwrap();
        let hwnd = window.handle().unwrap();
        let color = GetDlgItem(Some(hwnd), 100).unwrap();
        let screenshot = GetDlgItem(Some(hwnd), 200).unwrap();
        assert!(IsChild(hwnd, color).as_bool());
        assert!(IsChild(hwnd, screenshot).as_bool());
        SendMessageW(color, BM_CLICK, None, None);
        assert!(
            matches!(shared.borrow_mut().action.take(), Some((_, Action::Color(_, hex))) if hex == "#010203")
        );
        SendMessageW(screenshot, BM_CLICK, None, None);
        assert!(
            matches!(shared.borrow_mut().action.take(), Some((_, Action::Reuse(id))) if id == "id")
        );
        let delete = GetDlgItem(Some(hwnd), 300).unwrap();
        SendMessageW(delete, BM_CLICK, None, None);
        assert!(
            matches!(shared.borrow_mut().action.take(), Some((_, Action::Delete(id))) if id == "id")
        );
        SendMessageW(hwnd, WM_CLOSE, None, None);
        assert!(!IsWindowVisible(hwnd).as_bool());
        assert!(window.handle().is_some());
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(IsWindow(Some(source.0)).as_bool());
    }
}
#[test]
fn scroll_is_bounded_and_destroyed_owner_cannot_receive_late_actions() {
    unsafe {
        let source = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let origin = Origin {
            unit: crate::workspace_id::UnitId::parse("fixture").unwrap(),
            lifetime: Default::default(),
        };
        let window = Window::new(source.0, origin, Default::default()).unwrap();
        let hwnd = window.handle().unwrap();
        window.state.scroll.set(i32::MAX);
        layout::update(hwnd, &window.state).unwrap();
        assert!(window.state.scroll.get() < 5000);
        DestroyWindow(hwnd).unwrap();
        assert!(window.handle().is_none());
        assert!(window.state.view.borrow().action.is_none());
    }
}
