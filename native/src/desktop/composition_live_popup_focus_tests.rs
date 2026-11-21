//! 自有窗口验证焦点通知准入；不抢系统前台，不合成全局鼠标输入。
use super::*;
#[path = "composition_live_popup_control_tests.rs"]
mod control_tests;

struct Source(HWND);
impl Source {
    fn new(owner: Option<HWND>) -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Otter focus fixture"),
                WS_POPUP | WS_VISIBLE,
                -30000,
                -30000,
                32,
                32,
                owner,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}
fn live() -> (crate::live_frames::Source<Texture>, LiveWindow) {
    let frames = crate::live_frames::Source::new();
    let window = LiveWindow::new(frames.subscribe().unwrap(), 64, 64).unwrap();
    window.popup_watch().unwrap();
    window.allow_input().unwrap();
    window.request_default_input();
    window.take_input_request();
    window.input_status(true, false, None);
    (frames, window)
}
fn lose_focus(window: &LiveWindow, next: HWND) {
    unsafe {
        handle(
            window.hwnd,
            &window.state,
            WM_KILLFOCUS,
            WPARAM(next.0 as usize),
            LPARAM(0),
        );
    }
}

#[test]
fn source_popup_focus_is_not_cancelled_inside_window_callback() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    let (_frames, window) = live();
    lose_focus(&window, popup.0);
    assert!(
        window.focused_source_popup().is_none(),
        "pending is not authorized"
    );
    assert!(
        window.state.input.borrow().enabled,
        "source popup identity must be resolved outside WndProc before cancelling its modal call"
    );
    window.resolve_popup_focus_at(Some(&Target::bind(root.0).unwrap()), popup.0);
    assert!(window.state.input.borrow().enabled);
    assert!(window.state.popup_focus.borrow().active.is_some());
    assert_eq!(
        window
            .focused_source_popup()
            .unwrap()
            .target()
            .unwrap()
            .hwnd(),
        popup.0
    );
    window.state.input.borrow_mut().pause();
    assert!(
        window.focused_source_popup().is_none(),
        "paused input cannot seed discovery"
    );
}

#[test]
fn unrelated_and_ownerless_foreground_are_rejected_before_input_processing() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let other = Source::new(None);
    let unrelated_popup = Source::new(Some(other.0));
    let target = Target::bind(root.0).unwrap();
    for next in [other.0, unrelated_popup.0, root.0] {
        let (_frames, window) = live();
        lose_focus(&window, next);
        assert!(
            window.state.input.borrow().enabled,
            "WndProc only records a candidate"
        );
        window.resolve_popup_focus_at(Some(&target), next);
        assert!(!window.state.input.borrow().enabled);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(
            !window.popup_control_allowed(),
            "suspended parent cannot authorize child input"
        );
    }
}

#[test]
fn active_source_handoff_detects_external_focus_without_another_b_notification() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    let other = Source::new(None);
    let target = Target::bind(root.0).unwrap();
    let (_frames, window) = live();
    let (_peer_frames, peer) = live();
    window.popup_peers(vec![peer.hwnd]);
    lose_focus(&window, popup.0);
    window.resolve_popup_focus_at(Some(&target), popup.0);
    assert!(window.state.input.borrow().enabled);
    window.resolve_popup_focus_at(Some(&target), other.0);
    assert!(!window.state.input.borrow().enabled);
    unsafe {
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(peer.hwnd),
            WM_CANCELMODE,
            WM_CANCELMODE,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
    }
    assert!(!peer.state.input.borrow().enabled);
}

#[test]
fn source_popup_owner_change_or_hide_revokes_the_handoff() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let other = Source::new(None);
    let target = Target::bind(root.0).unwrap();
    for hide in [false, true] {
        let popup = Source::new(Some(root.0));
        let (_frames, window) = live();
        lose_focus(&window, popup.0);
        window.resolve_popup_focus_at(Some(&target), popup.0);
        assert!(window.state.input.borrow().enabled);
        unsafe {
            if hide {
                let _ = ShowWindow(popup.0, SW_HIDE);
            } else {
                SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, other.0 .0 as isize);
            }
        }
        window.resolve_popup_focus_at(Some(&target), popup.0);
        assert!(!window.state.input.borrow().enabled);
        assert!(window.state.popup_focus.borrow().active.is_none());
    }
}

#[test]
fn stale_focus_candidate_and_missing_source_authority_cannot_bypass_pause() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    let other = Source::new(None);
    let target = Target::bind(root.0).unwrap();
    for (authority, foreground) in [(Some(&target), other.0), (None, popup.0)] {
        let (_frames, window) = live();
        lose_focus(&window, popup.0);
        window.resolve_popup_focus_at(authority, foreground);
        assert!(!window.state.input.borrow().enabled);
    }
}

#[test]
fn missing_pointer_full_queue_remote_and_unarmed_windows_keep_immediate_pause() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    for mode in 0..4 {
        let (_frames, window) = live();
        match mode {
            0 => window.state.input.borrow_mut().buttons = 1,
            1 => {
                for code in 1..=32 {
                    assert!(window
                        .state
                        .input
                        .borrow_mut()
                        .push(Event::Key { code, down: true }));
                }
            }
            2 => window.state.input.borrow_mut().remote = true,
            _ => window.state.popup_focus.borrow_mut().armed = false,
        }
        lose_focus(&window, popup.0);
        assert!(!window.state.input.borrow().enabled);
        assert!(window.state.popup_focus.borrow().pending.is_none());
        assert_eq!(window.state.input.borrow().held_keys, [0; 4]);
    }
}

#[test]
fn repeated_key_down_then_release_does_not_leave_a_sticky_focus_guard() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    let (_frames, window) = live();
    for down in [true, true, false] {
        assert!(window
            .state
            .input
            .borrow_mut()
            .push(Event::Key { code: 65, down }));
    }
    lose_focus(&window, popup.0);
    window.resolve_popup_focus_at(Some(&Target::bind(root.0).unwrap()), popup.0);
    assert!(window.state.input.borrow().enabled);
    assert!(window.state.popup_focus.borrow().active.is_some());
}

#[test]
fn explicit_cancel_and_return_to_mirror_do_not_resurrect_or_cancel_control() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    let target = Target::bind(root.0).unwrap();
    let (_frames, window) = live();
    lose_focus(&window, popup.0);
    window.resolve_popup_focus_at(Some(&target), popup.0);
    window.resolve_popup_focus_at(Some(&target), window.hwnd);
    assert!(window.state.input.borrow().enabled);
    assert!(window.state.popup_focus.borrow().active.is_none());
    lose_focus(&window, popup.0);
    unsafe {
        cancel(window.hwnd, &window.state);
    }
    window.resolve_popup_focus_at(Some(&target), popup.0);
    assert!(!window.state.input.borrow().enabled);
    assert!(!window.state.input.borrow().desired);
}

#[test]
fn mirror_focus_handoff_releases_keys_in_order_without_cancelling_parent_modal() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_frames, window) = live();
    let (_peer_frames, peer) = live();
    window.popup_peers(vec![peer.hwnd]);
    let down = Event::Key {
        code: 65,
        down: true,
    };
    let up = Event::Key {
        code: 65,
        down: false,
    };
    assert!(window.state.input.borrow_mut().push(down));
    lose_focus(&window, peer.hwnd);
    assert!(window.state.input.borrow().enabled);
    assert!(window.popup_control_allowed());
    assert_eq!(window.take_input_request(), None);
    assert_eq!(window.pop_input(), Some(down));
    assert_eq!(window.pop_input(), Some(up));
    assert_eq!(window.state.input.borrow().held_keys, [0; 4]);
    // 第二条失活通知不得再次制造 up，也不发送 WM_CANCELMODE 给父/兄弟镜像。
    lose_focus(&window, peer.hwnd);
    assert_eq!(window.pop_input(), None);
    // 同一物理手势的 up 无论迟到原 B 还是新 B，都不能在交接释放后再送一次。
    assert!(window.state.input.borrow_mut().push(up));
    assert!(peer.state.input.borrow_mut().push(up));
    assert_eq!(window.pop_input(), None);
    assert_eq!(peer.pop_input(), None);
    unsafe {
        let mut message = MSG::default();
        assert!(!PeekMessageW(
            &mut message,
            Some(peer.hwnd),
            WM_CANCELMODE,
            WM_CANCELMODE,
            PM_REMOVE
        )
        .as_bool());
    }
}

#[test]
fn mirror_handoff_does_not_bypass_held_mouse_or_release_queue_capacity() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    for mouse in [true, false] {
        let (_frames, window) = live();
        let (_peer_frames, peer) = live();
        window.popup_peers(vec![peer.hwnd]);
        if mouse {
            window.state.input.borrow_mut().buttons = 1;
        } else {
            for code in 1..=32 {
                assert!(window
                    .state
                    .input
                    .borrow_mut()
                    .push(Event::Key { code, down: true }));
            }
        }
        lose_focus(&window, peer.hwnd);
        assert!(!window.state.input.borrow().enabled);
        assert_eq!(window.take_input_request(), Some(false));
        assert_eq!(window.state.input.borrow().held_keys, [0; 4]);
    }
}

#[path = "composition_live_popup_capture_tests.rs"]
mod capture_tests;
