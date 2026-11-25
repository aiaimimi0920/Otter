//! Isolation between yellow corners/local shortcuts and source input; real owned HWNDs and synthetic frames.
use super::*;
#[test]
fn yellow_corner_selects_pin_releases_source_and_escape_recycles() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        window.request_default_input();
        window.take_input_request();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(true, false, None);
        let corner = LPARAM(126 | (126 << 16));
        SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(corner));
        SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(corner));
        assert!(window.state.pin.selected.get());
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(0x45)), None);
        assert_eq!(window.take_pin_request().map(|key| key.0), Some(0x45));
        assert!(window.pop_input().is_none());
        window.enable_recycle_close();
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        let mut prior = [0; 256];
        GetKeyboardState(&mut prior).unwrap();
        for key in [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN] {
            let mut keys = [0; 256];
            keys[key.0 as usize] = 0x80;
            SetKeyboardState(&keys).unwrap();
            SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(0x1b)), None);
            assert!(
                !window.take_recycle_request(),
                "modified Escape cannot delete"
            );
        }
        SetKeyboardState(&[0; 256]).unwrap();
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(0x1b)), None);
        SetKeyboardState(&prior).unwrap();
        assert!(window.take_recycle_request());
        assert!(
            window.is_alive(),
            "recycle must retain capture until checkpoint commit"
        );
    }
}

#[test]
fn content_click_leaves_yellow_selection_and_resumes_source() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        window.request_default_input();
        window.take_input_request();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        pin::select(window.hwnd, &window.state);
        window.take_input_request();
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDOWN,
            None,
            Some(LPARAM(64 | (64 << 16))),
        );
        assert!(!window.state.pin.selected.get());
        assert_eq!(window.take_input_request(), Some(true));
        assert!(matches!(
            window.pop_input(),
            Some(crate::live_input::Event::Button { down: true, .. })
        ));
    }
}
