//! 实际 HWND 键盘消息验证焦点、按键时机及旧候选失效；不发送外部动作。
use super::*;
#[test]
fn surface_edit_policy_scene_update_preserves_valid_focus_but_never_old_candidate() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let window = buttons();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        focus(&window);
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        let old = window.take_click().unwrap();
        window.set_submission_enabled(false).unwrap();
        window.replace(Scene::parse(br#"{"id":"a","type":"button","props":{"label":"Updated"},"events":{"click":"updated"}}"#).unwrap()).unwrap();
        assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
        assert!(window
            .click_events_for_candidate(old, &binding(), 2)
            .is_err());
        window.set_submission_enabled(true).unwrap();
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        let events = window
            .click_events_for_candidate(window.take_click().unwrap(), &binding(), 2)
            .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["action"], "updated");
        window
            .replace(
                Scene::parse(br#"{"id":"a","type":"button","props":{"disabled":true}}"#).unwrap(),
            )
            .unwrap();
        assert!(window.state.keyboard.borrow().focused().is_none());
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        assert!(!window.has_pending_click());
    }
}
#[test]
fn surface_edit_policy_soft_submission_stop_keeps_focus_but_invalidates_candidates() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let window = buttons();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        focus(&window);
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        let old = window.take_click().unwrap();
        key(&window, WM_KEYDOWN, VK_SPACE.0, false);
        window.set_submission_enabled(false).unwrap();
        assert!(window.local_edit_enabled());
        assert!(!window.pointer_enabled());
        assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
        assert!(window.state.keyboard.borrow().pressed().is_none());
        assert!(window
            .click_events_for_candidate(old, &binding(), 1)
            .is_err());
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        assert!(!window.has_pending_click());
        window.set_submission_enabled(true).unwrap();
        assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
        key(&window, WM_KEYUP, VK_SPACE.0, false);
        assert!(!window.has_pending_click());
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        assert!(window.take_click().is_some());
        window.set_input_policy(false, true).unwrap();
        assert!(!window.local_edit_enabled());
        assert!(!window.pointer_enabled());
        assert!(window.state.keyboard.borrow().focused().is_none());
    }
}
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyboardState, SetFocus, SetKeyboardState, VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN, VK_RMENU, VK_RSHIFT, VK_RWIN,
    VK_SHIFT, VK_SPACE, VK_TAB,
};

// 只改变当前测试线程的键盘表，恢复完整256字节；不发送全局输入或改系统按键。
struct ThreadKeys([u8; 256]);
impl ThreadKeys {
    unsafe fn modifiers(held: &[VIRTUAL_KEY]) -> Self {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
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
            keys[key.0 as usize] &= 0x7f;
        }
        for key in held {
            keys[key.0 as usize] |= 0x80;
        }
        SetKeyboardState(&keys).unwrap();
        Self(original)
    }
}
impl Drop for ThreadKeys {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}

fn buttons() -> SurfaceWindow {
    SurfaceWindow::new(Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"a","type":"button","props":{"label":"A"},"events":{"click":"first"}},{"id":"disabled","type":"button","props":{"disabled":true}},{"id":"b","type":"button","props":{"label":"B"},"events":{"click":"second"}}]}"#).unwrap(), 240, 240).unwrap()
}
unsafe fn key(window: &SurfaceWindow, message: u32, key: u16, repeat: bool) {
    // 本夹具表达无修饰键输入，不继承交互桌面的 Ctrl/Alt/Win/Shift。
    let _neutral = ThreadKeys::modifiers(&[]);
    assert_eq!(
        GetFocus(),
        window.hwnd,
        "keyboard fixture lost thread focus before send"
    );
    raw_key(window, message, key, repeat);
    assert!(
        window.state.error.borrow().is_none(),
        "keyboard handler failed: {:?}",
        window.state.error.borrow()
    );
}
unsafe fn raw_key(window: &SurfaceWindow, message: u32, key: u16, repeat: bool) {
    SendMessageW(
        window.hwnd,
        message,
        Some(WPARAM(key as usize)),
        Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
    );
}
unsafe fn focus(window: &SurfaceWindow) {
    SetFocus(Some(window.hwnd)).unwrap();
    assert_eq!(GetFocus(), window.hwnd);
}
#[test]
fn surface_button_keyboard_tab_enter_space_and_repeat_use_single_slot() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let window = buttons();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        focus(&window);
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
        assert_eq!(window.state.keyboard.borrow().indicator(), Some("a"));
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.take_error().is_none());
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        let click = window.take_click().unwrap();
        let events = window
            .click_events_for_candidate(click, &binding(), 3)
            .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["action"], "first");
        key(&window, WM_KEYDOWN, VK_RETURN.0, true);
        assert!(!window.has_pending_click());
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("b"));
        key(&window, WM_KEYDOWN, VK_SPACE.0, false);
        assert!(!window.has_pending_click());
        assert_eq!(window.state.keyboard.borrow().pressed(), Some("b"));
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.take_error().is_none());
        key(&window, WM_KEYUP, VK_SPACE.0, false);
        assert!(window.state.keyboard.borrow().pressed().is_none());
        let click = window.take_click().unwrap();
        let events = window
            .click_events_for_candidate(click, &binding(), 4)
            .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["action"], "second");
        key(&window, WM_KEYUP, VK_SPACE.0, false);
        assert!(!window.has_pending_click());
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
    }
}
#[test]
fn surface_button_keyboard_rejects_changed_focus_and_cancels_space_on_disable() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let window = buttons();
    unsafe {
        focus(&window);
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        assert!(window.state.keyboard.borrow().focused().is_none());
        window.set_pointer_enabled(true).unwrap();
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        key(&window, WM_KEYDOWN, VK_RETURN.0, false);
        let candidate = window.take_click().unwrap();
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        // 即使切回同一 ID，也不能复活焦点切换前已经领取的候选。
        key(&window, WM_KEYDOWN, VK_TAB.0, false);
        assert!(window
            .click_events_for_candidate(candidate, &binding(), 1)
            .is_err());
        key(&window, WM_KEYDOWN, VK_SPACE.0, false);
        window.set_pointer_enabled(false).unwrap();
        window.set_pointer_enabled(true).unwrap();
        key(&window, WM_KEYUP, VK_SPACE.0, false);
        assert!(!window.has_pending_click());
        assert!(window.state.keyboard.borrow().focused().is_none());
    }
}

#[test]
fn surface_button_plain_keyboard_fixture_does_not_inherit_thread_shortcuts() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    for modifier in [VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN, VK_SHIFT] {
        let window = buttons();
        window.set_pointer_enabled(true).unwrap();
        unsafe {
            focus(&window);
            let _ambient = ThreadKeys::modifiers(&[modifier]);
            let actual = pointer::modifiers();
            assert!(actual.ctrl_key || actual.alt_key || actual.meta_key || actual.shift_key);
            key(&window, WM_KEYDOWN, VK_TAB.0, false);
            assert_eq!(
                window.state.keyboard.borrow().focused(),
                Some("a"),
                "plain Tab inherited ambient {modifier:?}"
            );
            key(&window, WM_KEYDOWN, VK_RETURN.0, false);
            assert!(window.take_click().is_some());
            let restored = pointer::modifiers();
            assert_eq!(
                (
                    actual.ctrl_key,
                    actual.alt_key,
                    actual.meta_key,
                    actual.shift_key
                ),
                (
                    restored.ctrl_key,
                    restored.alt_key,
                    restored.meta_key,
                    restored.shift_key
                )
            );
        }
    }
}

#[test]
fn surface_button_modified_keyboard_keeps_product_shortcut_admission() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let window = buttons();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        focus(&window);
        let _neutral = ThreadKeys::modifiers(&[]);
        raw_key(&window, WM_KEYDOWN, VK_TAB.0, false);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
        for modifier in [VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN] {
            let _held = ThreadKeys::modifiers(&[modifier]);
            raw_key(&window, WM_KEYDOWN, VK_TAB.0, false);
            assert_eq!(window.state.keyboard.borrow().focused(), Some("a"));
            raw_key(&window, WM_KEYDOWN, VK_RETURN.0, false);
            raw_key(&window, WM_KEYDOWN, VK_SPACE.0, false);
            assert!(window.take_click().is_none());
            assert!(window.state.keyboard.borrow().pressed().is_none());
        }
        {
            let _shift = ThreadKeys::modifiers(&[VK_SHIFT]);
            raw_key(&window, WM_KEYDOWN, VK_TAB.0, false);
            assert_eq!(window.state.keyboard.borrow().focused(), Some("b"));
        }
        assert_eq!(GetFocus(), window.hwnd);
        assert!(window.take_error().is_none());
    }
}

#[test]
fn surface_keyboard_fixture_restores_full_thread_state_on_unwind() {
    let mut original = [0; 256];
    unsafe {
        GetKeyboardState(&mut original).unwrap();
    }
    let result = std::panic::catch_unwind(|| unsafe {
        let _ambient = ThreadKeys::modifiers(&[VK_MENU, VK_RWIN]);
        let _neutral = ThreadKeys::modifiers(&[]);
        let modifiers = pointer::modifiers();
        assert!(
            !modifiers.alt_key
                && !modifiers.ctrl_key
                && !modifiers.meta_key
                && !modifiers.shift_key
        );
        std::panic::panic_any("expected keyboard state unwind probe");
    });
    assert_eq!(
        result.unwrap_err().downcast_ref::<&str>(),
        Some(&"expected keyboard state unwind probe")
    );
    let mut restored = [0; 256];
    unsafe {
        GetKeyboardState(&mut restored).unwrap();
    }
    assert_eq!(restored, original);
}
