//! Live 命令在 HWND 回调内只撤销源输入/排队；不注入物理键盘或激活其他窗口。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
struct Keys([u8; 256]);
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.0).unwrap();
        }
    }
}
#[test]
fn add_art_stops_source_key_queue_and_lock_hidden_destroy_never_leak_intent() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let source = crate::live_frames::Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        super::super::tests::publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.allow_input().unwrap();
        window.input_status(true, false, None);
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_SHIFT.0 as usize)),
            None,
        );
        assert!(matches!(
            window.pop_input(),
            Some(crate::live_input::Event::Key { down: true, .. })
        ));
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        let mut before = [0; 256];
        GetKeyboardState(&mut before).unwrap();
        let _keys = Keys(before);
        let mut held = [0; 256];
        held[VK_SHIFT.0 as usize] = 0x80;
        SetKeyboardState(&held).unwrap();
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(0x31)), None);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        assert!(window.take_art_request());
        SendMessageW(window.hwnd, WM_IME_CHAR, Some(WPARAM(0xff01)), None);
        assert!(!window.take_art_request());
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(0x31)),
            Some(LPARAM(1 << 30)),
        );
        assert!(!window.art_pending());
        for (locked, hidden) in [(true, false), (false, true)] {
            window.state.group_locked.set(locked);
            window.state.group_hidden.set(hidden);
            SendMessageW(window.hwnd, WM_SYSCOMMAND, Some(WPARAM(ADD)), None);
            assert!(!window.art_pending());
        }
        window.state.group_hidden.set(false);
        SendMessageW(window.hwnd, WM_SYSCOMMAND, Some(WPARAM(ADD)), None);
        assert!(window.art_pending());
        DestroyWindow(window.hwnd).unwrap();
        assert!(!window.art_pending());
        assert!(window.art_dialog_owner().is_none());
    }
}
