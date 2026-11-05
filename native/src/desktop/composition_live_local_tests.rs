//! 当前线程键盘快照和隐藏 HWND 验证，不向桌面注入输入。
use super::*;
use crate::{desktop::Apartment, live_frames::Source};
struct Keys([u8; 256]);
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}
pub(in crate::desktop) unsafe fn with_keys(modifiers: &[VIRTUAL_KEY], action: impl FnOnce()) {
    let mut original = [0; 256];
    GetKeyboardState(&mut original).unwrap();
    let _restore = Keys(original);
    let mut keys = original;
    for key in [
        VK_CONTROL,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_MENU,
        VK_LMENU,
        VK_RMENU,
        VK_SHIFT,
        VK_LSHIFT,
        VK_RSHIFT,
        VK_LWIN,
        VK_RWIN,
    ] {
        keys[key.0 as usize] = 0;
    }
    for key in modifiers {
        keys[key.0 as usize] = 0x80;
    }
    SetKeyboardState(&keys).unwrap();
    action();
}
pub(in crate::desktop) unsafe fn wheel(
    hwnd: HWND,
    modifiers: &[VIRTUAL_KEY],
    delta: i16,
    pointer: (i32, i32),
) {
    with_keys(modifiers, || {
        SendMessageW(
            hwnd,
            WM_MOUSEWHEEL,
            Some(WPARAM((delta as u16 as usize) << 16)),
            Some(LPARAM(
                ((pointer.0 as u16 as u32) | ((pointer.1 as u16 as u32) << 16)) as isize,
            )),
        );
    });
}
#[test]
fn local_wheel_opacity_clamps_and_requires_exact_modifiers() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new_at(
            source.subscribe().unwrap(),
            RECT {
                left: -500,
                top: -500,
                right: -372,
                bottom: -372,
            },
        )
        .unwrap();
        wheel(window.hwnd, &[VK_MENU], -120, (-436, -436));
        assert!((window.opacity() - 0.88).abs() < 0.00001);
        let geometry = window.freeze_rect([2, 2]).unwrap();
        for modifiers in [
            &[][..],
            &[VK_CONTROL, VK_MENU][..],
            &[VK_MENU, VK_SHIFT][..],
            &[VK_MENU, VK_LWIN][..],
        ] {
            wheel(window.hwnd, modifiers, -120, (-436, -436));
            assert!((window.opacity() - 0.88).abs() < 0.00001);
            assert_eq!(window.freeze_rect([2, 2]).unwrap(), geometry);
        }
        wheel(window.hwnd, &[VK_MENU], -30, (-436, -436));
        assert!((window.opacity() - 0.85).abs() < 0.00001);
        for (delta, expected) in [(-2000, 0.0), (2000, 1.0)] {
            wheel(window.hwnd, &[VK_MENU], delta, (-436, -436));
            assert_eq!(window.opacity(), expected);
        }
        window.state.group_locked.set(true);
        wheel(window.hwnd, &[VK_MENU], -120, (-436, -436));
        assert_eq!(window.opacity(), 1.0);
        window.state.group_locked.set(false);
        window.state.group_hidden.set(true);
        wheel(window.hwnd, &[VK_MENU], -120, (-436, -436));
        assert_eq!(window.opacity(), 1.0);
    }
}
#[test]
fn local_opacity_during_host_borrow_is_reapplied_by_compensating_paint() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new_at(
            source.subscribe().unwrap(),
            RECT {
                left: -500,
                top: -500,
                right: -372,
                bottom: -372,
            },
        )
        .unwrap();
        super::super::tests::publish(&source);
        window
            .state
            .with_host(|_| {
                wheel(window.hwnd, &[VK_MENU], -120, (-436, -436));
                Ok(())
            })
            .unwrap();
        assert!((window.opacity() - 0.88).abs() < 0.00001);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        assert!(!window.state.deferred.get());
        assert!(
            (window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .applied_opacity_for_test()
                .unwrap()
                - 0.88)
                .abs()
                < 0.00001
        );
        assert!(window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presented_viewport([128, 128])
            .is_some());
    }
}
#[test]
fn local_zoom_anchors_client_and_cancels_source_queue() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new_at(
            source.subscribe().unwrap(),
            RECT {
                left: -500,
                top: -500,
                right: -372,
                bottom: -372,
            },
        )
        .unwrap();
        super::super::tests::publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        let before = window.freeze_rect([2, 2]).unwrap();
        let pointer = (before.x + 32, before.y + 64);
        let expected = crate::pin_geometry::zoom(before, pointer, 1.0).unwrap();
        window.input_status(true, false, None);
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_CONTROL.0 as usize)),
            None,
        );
        wheel(window.hwnd, &[VK_CONTROL], 120, pointer);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        assert_eq!(window.freeze_rect([2, 2]).unwrap(), expected);
        assert_eq!(window.opacity(), 1.0);
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDOWN,
            None,
            Some(LPARAM(32 | (32 << 16))),
        );
        assert_eq!(GetCapture(), window.hwnd);
        wheel(window.hwnd, &[VK_MENU], -120, pointer);
        assert_ne!(GetCapture(), window.hwnd);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        assert!((window.opacity() - 0.88).abs() < 0.00001);
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        wheel(window.hwnd, &[], 120, pointer);
        assert!(matches!(
            window.pop_input(),
            Some(crate::live_input::Event::Wheel { .. })
        ));
    }
}
