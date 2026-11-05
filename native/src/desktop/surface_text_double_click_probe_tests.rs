//! 系统生成双击消息，再沿正式词级拖动路径；只向实际命中的自有窗口注入。
use super::*;
#[test]
#[ignore = "requires explicit owned visible desktop input probe"]
fn text_selection_system_double_click_probe() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_TEXT_SELECTION_PROBE").as_deref(),
        Ok("1")
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        for key in [
            VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN,
        ] {
            assert!(GetAsyncKeyState(key.0 as i32) >= 0, "desktop input is busy");
        }
        assert!(GetCapture().is_invalid());
        let window = window();
        let observer = Observer::new(window.hwnd);
        window.show().unwrap();
        SetWindowPos(
            window.hwnd,
            Some(HWND_TOPMOST),
            400,
            300,
            260,
            180,
            SWP_SHOWWINDOW,
        )
        .unwrap();
        let activated = SetForegroundWindow(window.hwnd).as_bool();
        let immediate = GetForegroundWindow();
        pump();
        let foreground = GetForegroundWindow();
        println!("double-click preflight: requested={:?}; activated={activated}; immediate={immediate:?}; after_pump={foreground:?}; injected=false", window.hwnd);
        assert_eq!(
            foreground, window.hwnd,
            "owned foreground guard rejected injection"
        );
        let mut start = POINT { x: 10, y: 8 };
        let mut end = POINT { x: 210, y: 8 };
        assert!(ClientToScreen(window.hwnd, &mut start).as_bool());
        assert!(ClientToScreen(window.hwnd, &mut end).as_bool());
        assert_eq!(WindowFromPoint(start), window.hwnd);
        assert_eq!(WindowFromPoint(end), window.hwnd);
        let mut original = POINT::default();
        GetCursorPos(&mut original).unwrap();
        // 一批包含完整两次点击和最终抬起；第二次保持按下后再移动，验证词级拖动。
        send_inputs(&[
            mouse(start, MOUSE_EVENT_FLAGS(0)),
            mouse(start, MOUSEEVENTF_LEFTDOWN),
            mouse(start, MOUSEEVENTF_LEFTUP),
            mouse(start, MOUSEEVENTF_LEFTDOWN),
            mouse(end, MOUSEEVENTF_MOVE_NOCOALESCE),
            mouse(end, MOUSEEVENTF_LEFTUP),
        ]);
        let deadline = Instant::now() + Duration::from_millis(400);
        let mut clicks = 0;
        let mut word_at_double = None;
        let mut captured_at_double = false;
        while Instant::now() < deadline {
            let mut msg = MSG::default();
            for _ in 0..128 {
                if !PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
                if msg.hwnd == window.hwnd && msg.message == WM_LBUTTONDBLCLK {
                    word_at_double = selected(&window);
                    captured_at_double = GetCapture() == window.hwnd;
                }
                // 模拟正常宿主在消息之间消费单槽 click，但不发送远端动作。
                if window.take_click().is_some() {
                    clicks += 1;
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let mut current = POINT::default();
        GetCursorPos(&mut current).unwrap();
        if current.x == end.x && current.y == end.y && GetForegroundWindow() == window.hwnd {
            SetCursorPos(original.x, original.y).unwrap();
        }
        println!("double-click trace={:?}; initial={word_at_double:?}; captured={captured_at_double}; final={:?}; clicks={clicks}", observer.trace, selected(&window));
        assert!(!observer.trace.overflow);
        let edges: Vec<_> = observer
            .trace
            .messages
            .iter()
            .copied()
            .filter(|m| *m != WM_MOUSEMOVE)
            .collect();
        assert_eq!(
            edges,
            [WM_LBUTTONDOWN, WM_LBUTTONUP, WM_LBUTTONDBLCLK, WM_LBUTTONUP]
        );
        assert_eq!(word_at_double.as_deref(), Some("Native "));
        assert!(captured_at_double);
        assert_eq!(selected(&window).as_deref(), Some("Native selection"));
        assert_eq!(clicks, 1);
        assert_ne!(GetCapture(), window.hwnd);
        assert!(!window.has_pending_click());
        assert!(!window.has_pending_edit());
        assert!(window.take_error().is_none());
    }
}
