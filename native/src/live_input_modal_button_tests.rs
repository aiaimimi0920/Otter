//! 自有嵌套消息泵：按钮各阶段准入、正常松开及 held 取消必须恰好投递一次。
use super::*;

#[test]
#[ignore = "owned hidden native windows; nested menu message loop"]
fn local_right_release_is_pending_until_modal_closes_without_replay() {
    assert_modal_button(WM_RBUTTONUP, false);
}

#[test]
#[ignore = "owned hidden native windows; left release opens nested modal dialog"]
fn local_left_release_is_pending_until_modal_closes_without_replay() {
    assert_modal_button(WM_LBUTTONUP, false);
}

#[test]
#[ignore = "owned hidden native windows; left press opens nested modal menu"]
fn local_left_press_keeps_control_and_releases_once() {
    assert_modal_button(WM_LBUTTONDOWN, false);
}

#[test]
#[ignore = "owned hidden native windows; double click opens nested modal dialog"]
fn local_left_double_click_keeps_control_and_releases_once() {
    assert_modal_button(WM_LBUTTONDBLCLK, false);
}

#[test]
#[ignore = "owned hidden native windows; held modal press cleanup"]
fn cancelling_pending_left_press_releases_before_cancelmode() {
    for trigger in [WM_LBUTTONDOWN, WM_LBUTTONDBLCLK] {
        assert_modal_button(trigger, true);
    }
}

fn assert_modal_button(trigger: u32, cancel_held: bool) {
    let (mut fixture, target, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        for (name, value) in [
            (w!("Otter.Test.ModalTrigger"), trigger as usize),
            (w!("Otter.Test.ModalNext"), 1),
            (w!("Otter.Test.ModalCancelMode"), 1),
        ] {
            SetPropW(hwnd, name, Some(HANDLE(value as *mut _))).unwrap();
        }
    }
    let button = if trigger == WM_RBUTTONUP {
        Button::Right
    } else {
        Button::Left
    };
    let event = |down| Event::Button {
        point: [0.25; 2],
        button,
        down,
        double: down && trigger == WM_LBUTTONDBLCLK,
    };
    let opens_on_press = matches!(trigger, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK);
    let start = Instant::now();
    let press = controller.dispatch_status(1, event(true));
    let result = if opens_on_press {
        press
    } else {
        assert_eq!(press, Ok(live_input::Delivery::Completed));
        controller.dispatch_status(2, event(false))
    };
    let elapsed = start.elapsed();
    assert_eq!(result, Ok(live_input::Delivery::Pending));
    assert!(!unsafe { GetPropW(hwnd, w!("Otter.Test.ModalActive")) }
        .0
        .is_null());
    assert!(elapsed < Duration::from_secs(1));
    assert!(controller.enabled());
    assert_eq!(
        controller.dispatch_status(3, Event::Move { point: [0.25; 2] }),
        Ok(live_input::Delivery::Completed)
    );

    let cleanup = if cancel_held {
        controller.cancel()
    } else {
        if opens_on_press {
            assert_eq!(
                controller.dispatch_status(4, event(false)),
                Ok(live_input::Delivery::Completed)
            );
            assert!(
                controller.pending_delivery(),
                "release must not replace the opening callback"
            );
        }
        unsafe { PostMessageW(Some(hwnd), CLOSE_MODAL, WPARAM(0), LPARAM(0)).unwrap() };
        let deadline = Instant::now() + Duration::from_secs(1);
        while controller.pending_delivery() && Instant::now() < deadline {
            controller.idle().unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            !controller.pending_delivery(),
            "final callback must retire the slot"
        );
        controller.cancel()
    };
    let records = fixture.finish();
    let release = if button == Button::Left {
        WM_LBUTTONUP
    } else {
        WM_RBUTTONUP
    };
    assert_eq!(cleanup.failed, 0);
    assert_eq!(cleanup.released, u8::from(cancel_held));
    assert_eq!(records.iter().filter(|r| r.1 == trigger).count(), 1);
    assert_eq!(records.iter().filter(|r| r.1 == release).count(), 1);
    if cancel_held {
        let released = records.iter().position(|r| r.1 == release).unwrap();
        let cancelled = records.iter().position(|r| r.1 == WM_CANCELMODE).unwrap();
        assert!(released < cancelled);
        assert_eq!(records.iter().filter(|r| r.1 == WM_CANCELMODE).count(), 1);
    }
}
