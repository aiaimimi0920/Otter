//! Owned Win32 mock-class gate: verifies the message chain, not real CLR/WinForms compatibility acceptance.
use super::*;

#[test]
#[ignore = "interactive desktop; owned simulated button class, no global input"]
fn actual_occluded_button_notifies_once_but_drag_and_cleanup_do_not() {
    let (mut fixture, root, child) = fixture_with_button_class(true);
    let root_hwnd = root.hwnd().0 as usize;
    let mut controller = Controller::new(Backend::new(root, None).unwrap());
    controller.enable().unwrap();
    let edge = |down| Event::Button {
        point: [0.2; 2],
        button: Button::Left,
        down,
        double: false,
    };
    controller.dispatch(1, edge(true)).unwrap();
    controller.dispatch(2, edge(false)).unwrap();
    controller.dispatch(3, edge(true)).unwrap();
    controller
        .dispatch(4, Event::Move { point: [0.5; 2] })
        .unwrap();
    controller
        .dispatch(5, Event::Move { point: [0.2; 2] })
        .unwrap();
    controller.dispatch(6, edge(false)).unwrap();
    controller.dispatch(7, edge(true)).unwrap();
    assert_eq!(controller.cancel().released, 1);
    let records = fixture.finish();
    let notifications: Vec<_> = records
        .iter()
        .filter(|(_, msg, _, _)| *msg == WM_COMMAND)
        .collect();
    assert_eq!(notifications.len(), 1, "{records:?}");
    assert_eq!(notifications[0].0, root_hwnd);
    assert_eq!(notifications[0].3, child as isize);
    assert_eq!(
        records
            .iter()
            .filter(|(hwnd, msg, _, _)| *hwnd == child && *msg == WM_LBUTTONDOWN)
            .count(),
        3
    );
    assert_eq!(
        records
            .iter()
            .filter(|(hwnd, msg, _, _)| *hwnd == child && *msg == WM_LBUTTONUP)
            .count(),
        3
    );
}

#[test]
#[ignore = "owned simulated WinForms class; modal notification and Escape"]
fn local_button_modal_notification_completes_without_replaying_click() {
    modal_notification(false);
}

#[test]
#[ignore = "owned simulated WinForms class; cancellation of modal notification"]
fn local_button_modal_notification_cancel_does_not_repeat_mouse_release() {
    modal_notification(true);
}

fn modal_notification(cancel: bool) {
    let (mut fixture, root, child) = fixture_with_button_class(true);
    let hwnd = root.hwnd();
    let mut controller = Controller::new(Backend::new_local(root, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        for (name, value) in [
            (w!("Otter.Test.ModalTrigger"), WM_COMMAND as usize),
            (w!("Otter.Test.ModalNext"), 1),
            (w!("Otter.Test.ModalCancelMode"), 1),
        ] {
            SetPropW(hwnd, name, Some(HANDLE(value as *mut _))).unwrap();
        }
    }
    let edge = |down| Event::Button {
        point: [0.2; 2],
        button: Button::Left,
        down,
        double: false,
    };
    controller.dispatch(1, edge(true)).unwrap();
    let result = controller.dispatch_status(2, edge(false));
    assert_eq!(result, Ok(live_input::Delivery::Pending));
    assert!(controller.enabled());
    assert!(!unsafe { GetPropW(hwnd, w!("Otter.Test.ModalActive")) }
        .0
        .is_null());
    if !cancel {
        // Escape must remain bound to the notification recipient (parent window), not the last-clicked child button.
        controller
            .dispatch(
                3,
                Event::Key {
                    code: 0x1b,
                    down: true,
                },
            )
            .unwrap();
        controller
            .dispatch(
                4,
                Event::Key {
                    code: 0x1b,
                    down: false,
                },
            )
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while controller.pending_delivery() && std::time::Instant::now() < deadline {
            controller.idle().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!controller.pending_delivery());
    }
    let cleanup = controller.cancel();
    let records = fixture.finish();
    assert_eq!(cleanup.failed, 0);
    assert_eq!(cleanup.released, 0);
    let notifications: Vec<_> = records.iter().filter(|r| r.1 == WM_COMMAND).collect();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].0, hwnd.0 as usize);
    assert_eq!(notifications[0].3, child as isize);
    for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
        assert_eq!(
            records
                .iter()
                .filter(|r| r.0 == child && r.1 == message)
                .count(),
            1
        );
    }
    if cancel {
        assert_eq!(
            records
                .iter()
                .filter(|r| r.0 == hwnd.0 as usize && r.1 == WM_CANCELMODE)
                .count(),
            1
        );
    } else {
        for message in [WM_KEYDOWN, WM_KEYUP] {
            assert_eq!(
                records
                    .iter()
                    .filter(|r| r.0 == hwnd.0 as usize && r.1 == message && r.2 == 0x1b)
                    .count(),
                1
            );
        }
    }
}
