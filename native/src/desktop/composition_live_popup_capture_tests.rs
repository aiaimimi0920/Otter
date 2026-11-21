//! 源 dialog 在 down 内激活：先排原目标 up，再验证来源，不用取消父模态收尾。
use super::*;
use crate::live_input::Button;

fn button(button: Button, down: bool) -> Event {
    Event::Button {
        point: [0.25, 0.5],
        button,
        down,
        double: false,
    }
}
fn hold(window: &LiveWindow, button: Button, mask: u8) {
    let mut input = window.state.input.borrow_mut();
    assert!(input.button_edge(mask, true));
    assert!(input.push(self::button(button, true)));
}

#[test]
fn down_opened_source_dialog_releases_all_buttons_and_keys_in_original_fifo() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let popup = Source::new(Some(root.0));
    let (_frames, window) = live();
    let buttons = [(Button::Left, 1), (Button::Right, 2), (Button::Middle, 4)];
    for (button, mask) in buttons {
        hold(&window, button, mask);
    }
    let down = Event::Key {
        code: 65,
        down: true,
    };
    assert!(window.state.input.borrow_mut().push(down));
    lose_focus(&window, popup.0);
    assert!(window.state.input.borrow().enabled);
    assert_eq!(
        window.take_input_request(),
        None,
        "handoff must not cancel the worker/modal"
    );
    for (button, _) in buttons {
        assert_eq!(window.pop_input(), Some(self::button(button, true)));
    }
    assert_eq!(window.pop_input(), Some(down));
    for (button, _) in buttons {
        assert_eq!(window.pop_input(), Some(self::button(button, false)));
    }
    assert_eq!(
        window.pop_input(),
        Some(Event::Key {
            code: 65,
            down: false
        })
    );
    assert_eq!(window.pop_input(), None);
    assert_eq!(window.state.input.borrow().buttons, 0);
    assert_eq!(window.state.input.borrow().held_keys, [0; 4]);
    window.resolve_popup_focus_at(Some(&Target::bind(root.0).unwrap()), popup.0);
    assert!(window.state.input.borrow().enabled);
    assert!(window.state.popup_focus.borrow().active.is_some());
    // 同一物理按住的迟到 up 被原窗口边沿准入拒绝，新的完整手势仍可进入。
    for (_, mask) in buttons {
        assert!(!window.state.input.borrow_mut().button_edge(mask, false));
    }
    lose_focus(&window, popup.0);
    assert_eq!(
        window.pop_input(),
        None,
        "repeated notification cannot enqueue duplicate ups"
    );
    hold(&window, Button::Left, 1);
    assert_eq!(window.pop_input(), Some(button(Button::Left, true)));
    assert!(window.state.input.borrow_mut().button_edge(1, false));
    assert!(window
        .state
        .input
        .borrow_mut()
        .push(button(Button::Left, false)));
    assert_eq!(window.pop_input(), Some(button(Button::Left, false)));
}

#[test]
fn release_uses_latest_admitted_pointer_and_unrelated_focus_still_pauses() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let other = Source::new(None);
    let (_frames, window) = live();
    hold(&window, Button::Left, 1);
    let point = [0.75, 0.25];
    assert!(window.state.input.borrow_mut().push(Event::Move { point }));
    lose_focus(&window, other.0);
    assert!(
        window.state.input.borrow().enabled,
        "only the old target's release is queued before authority check"
    );
    assert_eq!(window.pop_input(), Some(button(Button::Left, true)));
    assert_eq!(window.pop_input(), Some(Event::Move { point }));
    assert_eq!(
        window.pop_input(),
        Some(Event::Button {
            point,
            button: Button::Left,
            down: false,
            double: false
        })
    );
    window.resolve_popup_focus_at(Some(&Target::bind(root.0).unwrap()), other.0);
    assert!(!window.state.input.borrow().enabled);
    assert_eq!(window.take_input_request(), Some(false));
}

#[test]
fn capture_loss_only_defers_to_a_nonmirror_foreground_then_rechecks_ownership() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let target = Target::bind(root.0).unwrap();
    let popup = Source::new(Some(root.0));
    let unrelated = Source::new(None);
    for next in [popup.0, unrelated.0] {
        let (_frames, window) = live();
        hold(&window, Button::Left, 1);
        assert!(!unsafe {
            focus_handoff_to(window.hwnd, &window.state, WM_CAPTURECHANGED, window.hwnd)
        });
        assert_eq!(window.state.input.borrow().buttons, 1);
        assert!(unsafe { focus_handoff_to(window.hwnd, &window.state, WM_CAPTURECHANGED, next) });
        assert_eq!(window.pop_input(), Some(button(Button::Left, true)));
        assert_eq!(window.pop_input(), Some(button(Button::Left, false)));
        window.resolve_popup_focus_at(Some(&target), next);
        assert_eq!(window.state.input.borrow().enabled, next == popup.0);
    }
}

#[test]
fn release_overload_and_explicit_cancel_never_resurrect_pending_source_focus() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let root = Source::new(None);
    let target = Target::bind(root.0).unwrap();
    let popup = Source::new(Some(root.0));
    for overload in [true, false] {
        let (_frames, window) = live();
        hold(&window, Button::Left, 1);
        if overload {
            for code in 1..=31 {
                assert!(window
                    .state
                    .input
                    .borrow_mut()
                    .push(Event::Key { code, down: true }));
            }
        }
        lose_focus(&window, popup.0);
        if !overload {
            assert!(window.state.input.borrow().enabled);
            unsafe {
                cancel(window.hwnd, &window.state);
            }
        }
        window.resolve_popup_focus_at(Some(&target), popup.0);
        assert!(!window.state.input.borrow().enabled);
        assert_eq!(window.state.input.borrow().buttons, 0);
        assert_eq!(window.pop_input(), None);
        assert_eq!(window.take_input_request(), Some(false));
    }
}
