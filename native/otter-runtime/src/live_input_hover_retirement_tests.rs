//! Menu-destruction/hover-cleanup race: confirm only the terminal state of nonexistent hover, without faking successful held releases.
use super::*;

#[test]
fn leave_cleanup_accepts_destruction_during_delivery() {
    let (mut fixture, root, child) = fixture();
    let target = Destination::bind(&root, HWND(child as *mut _)).unwrap();
    unsafe {
        SetPropW(
            target.hwnd(),
            w!("Otter.Test.DestroyOnLeave"),
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap();
    }
    assert!(message::release_to_with_timeout(&target, Message::leave(), 250).is_ok());
    assert!(!unsafe { IsWindow(Some(target.hwnd())) }.as_bool());
    // Repeated hover cleanup of the same old target needs no delivery; ordinary input and held edges must never report success.
    assert!(message::release_to_with_timeout(&target, Message::leave(), 250).is_ok());
    assert!(message::send_to(&target, Message::motion([0, 0], 0).unwrap()).is_err());
    assert!(message::release_to(&target, Message::key(0x41, false).unwrap()).is_err());
    assert!(message::release_to(
        &target,
        Message::button([0, 0], 0, Button::Left, false, false).unwrap()
    )
    .is_err());
    fixture.finish();
}

#[test]
fn live_hover_timeout_is_still_a_cleanup_failure() {
    let (mut fixture, root, child) = fixture();
    let target = Destination::bind(&root, HWND(child as *mut _)).unwrap();
    unsafe {
        SetPropW(
            target.hwnd(),
            w!("Otter.Test.MotionDelay"),
            Some(HANDLE(150usize as *mut _)),
        )
        .unwrap();
    }
    assert!(message::release_to_with_timeout(&target, Message::leave(), 20).is_err());
    assert!(unsafe { IsWindow(Some(target.hwnd())) }.as_bool());
    fixture.finish();
}

#[test]
fn hidden_menu_has_no_leave_delivery_but_still_receives_held_release() {
    let (mut fixture, root, child) = fixture();
    let target = Destination::bind(&root, HWND(child as *mut _)).unwrap();
    unsafe {
        let _ = ShowWindow(target.hwnd(), SW_HIDE);
        SetPropW(
            target.hwnd(),
            w!("Otter.Test.MotionDelay"),
            Some(HANDLE(150usize as *mut _)),
        )
        .unwrap();
    }
    assert!(message::release_to_with_timeout(&target, Message::leave(), 20).is_ok());
    assert!(message::release_to(&target, Message::key(0x41, false).unwrap()).is_ok());
    let records = fixture.finish();
    assert!(!records
        .iter()
        .any(|record| record.1 == windows::Win32::UI::Controls::WM_MOUSELEAVE));
    assert!(records
        .iter()
        .any(|record| record.0 == child && record.1 == WM_KEYUP));
}

#[test]
fn disabled_native_root_ignores_only_motion_not_held_edges() {
    use crate::live_input::Backend as _;
    use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    let (mut fixture, root, _) = fixture();
    let mut backend = Backend::new_local(root.clone(), None).unwrap();
    let motion = Event::Move { point: [0.2; 2] };
    assert!(!backend.skip_modal_input(motion));
    unsafe {
        let _ = EnableWindow(root.hwnd(), false);
    }
    for event in [
        motion,
        Event::Hover { point: [0.2; 2] },
        Event::Wheel {
            point: [0.2; 2],
            delta: -120,
            horizontal: false,
        },
    ] {
        assert!(backend.skip_modal_input(event));
        assert_eq!(backend.delivery(), live_input::Delivery::IgnoredWhileModal);
    }
    for event in [
        Event::Leave,
        Event::Key {
            code: 65,
            down: false,
        },
        Event::Button {
            point: [0.2; 2],
            button: Button::Left,
            down: false,
            double: false,
        },
    ] {
        assert!(!backend.skip_modal_input(event));
    }
    assert!(!Backend::new(root.clone(), None)
        .unwrap()
        .skip_modal_input(motion));
    unsafe {
        let _ = EnableWindow(root.hwnd(), true);
    }
    assert!(!backend.skip_modal_input(motion));
    fixture.finish();
}

#[test]
fn disabled_chromium_class_is_not_filtered_as_native_motion() {
    use crate::live_input::Backend as _;
    use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    let (mut fixture, root, _) = fixture_with_class(false, true);
    unsafe {
        let _ = EnableWindow(root.hwnd(), false);
    }
    let mut backend = Backend::new_local(root, None).unwrap();
    for event in [
        Event::Move { point: [0.2; 2] },
        Event::Hover { point: [0.2; 2] },
        Event::Wheel {
            point: [0.2; 2],
            delta: -120,
            horizontal: false,
        },
    ] {
        assert!(!backend.skip_modal_input(event));
    }
    fixture.finish();
}
