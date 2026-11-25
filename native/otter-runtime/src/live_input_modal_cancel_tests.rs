//! 模态取消的完成观察：短延迟可结算，未完成仍失败，均不得重放右键。
use super::*;

#[test]
#[ignore = "owned hidden native windows; two Escape cycles close two menu levels"]
fn successive_escape_cycles_close_two_levels_and_cleanup_completes() {
    let (mut fixture, target, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        SetPropW(
            hwnd,
            w!("Otter.Test.ModalNext"),
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap();
        SetPropW(
            hwnd,
            w!("Otter.Test.ModalEscapeLevels"),
            Some(HANDLE(2usize as *mut _)),
        )
        .unwrap();
    }
    let button = |down| Event::Button {
        point: [0.25; 2],
        button: Button::Right,
        down,
        double: false,
    };
    let escape = |down| Event::Key { code: 0x1b, down };
    controller.dispatch(1, button(true)).unwrap();
    controller.dispatch(2, button(false)).unwrap();
    controller.dispatch(3, escape(true)).unwrap();
    controller.dispatch(4, escape(false)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while unsafe { GetPropW(hwnd, w!("Otter.Test.ModalEscapeLevels")) }.0 as usize != 1
        && Instant::now() < deadline
    {
        controller.idle().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        unsafe { GetPropW(hwnd, w!("Otter.Test.ModalEscapeLevels")) }.0 as usize,
        1
    );
    assert!(
        controller.pending_delivery(),
        "first Escape only closes the child menu"
    );
    controller.dispatch(5, escape(true)).unwrap();
    controller.dispatch(6, escape(false)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while controller.pending_delivery() && Instant::now() < deadline {
        controller.idle().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        !controller.pending_delivery(),
        "second Escape must close the parent menu"
    );
    assert_eq!(controller.cancel().failed, 0);
    let records = fixture.finish();
    let keys: Vec<_> = records
        .iter()
        .filter(|r| r.2 == 0x1b && matches!(r.1, WM_KEYDOWN | WM_KEYUP))
        .map(|r| r.1)
        .collect();
    assert_eq!(keys, [WM_KEYDOWN, WM_KEYUP, WM_KEYDOWN, WM_KEYUP]);
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 1);
}

#[test]
#[ignore = "owned hidden native windows; cancellation must end the original modal loop"]
fn cancellation_ends_owned_modal_without_escape_or_right_replay() {
    let (mut fixture, target, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        for property in [w!("Otter.Test.ModalNext"), w!("Otter.Test.ModalCancelMode")] {
            SetPropW(
                hwnd,
                property,
                Some(HANDLE(std::ptr::without_provenance_mut(1))),
            )
            .unwrap();
        }
    }
    let button = |down| Event::Button {
        point: [0.25; 2],
        button: Button::Right,
        down,
        double: false,
    };
    controller.dispatch(1, button(true)).unwrap();
    assert_eq!(
        controller.dispatch_status(2, button(false)),
        Ok(live_input::Delivery::Pending)
    );
    assert_eq!(controller.cancel().failed, 0);
    controller.enable().unwrap();
    controller.dispatch(3, button(true)).unwrap();
    controller.dispatch(4, button(false)).unwrap();
    assert_eq!(controller.cancel().failed, 0);
    let records = fixture.finish();
    assert_eq!(records.iter().filter(|r| r.1 == WM_CANCELMODE).count(), 1);
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 2);
    assert!(!records.iter().any(|r| matches!(r.1, WM_KEYDOWN | WM_KEYUP)));
}

#[test]
#[ignore = "owned hidden native windows; second Escape release must preserve posted queue order"]
fn second_escape_press_keeps_both_pairs_ordered_on_cancel() {
    let (mut fixture, target, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        for property in [w!("Otter.Test.ModalNext"), w!("Otter.Test.ModalPumpHold")] {
            SetPropW(
                hwnd,
                property,
                Some(HANDLE(std::ptr::without_provenance_mut(1))),
            )
            .unwrap();
        }
    }
    let button = |down| Event::Button {
        point: [0.25; 2],
        button: Button::Right,
        down,
        double: false,
    };
    let escape = |down| Event::Key { code: 0x1b, down };
    controller.dispatch(1, button(true)).unwrap();
    assert_eq!(
        controller.dispatch_status(2, button(false)),
        Ok(live_input::Delivery::Pending)
    );
    controller.dispatch(3, escape(true)).unwrap();
    controller.dispatch(4, escape(false)).unwrap();
    assert_eq!(
        controller.dispatch_status(5, escape(true)),
        Ok(live_input::Delivery::Pending)
    );
    // 新周期欠下的 up 也必须 posted，不能越过已入队的第一对和第二次 down。
    assert!(controller.cancel().failed > 0);
    unsafe {
        RemovePropW(hwnd, w!("Otter.Test.ModalPumpHold")).unwrap();
    }
    let records = fixture.finish();
    let keys: Vec<_> = records
        .iter()
        .filter(|r| r.2 == 0x1b && matches!(r.1, WM_KEYDOWN | WM_KEYUP))
        .map(|r| r.1)
        .collect();
    assert_eq!(keys, [WM_KEYDOWN, WM_KEYUP, WM_KEYDOWN, WM_KEYUP]);
}

#[test]
#[ignore = "owned hidden native windows; synchronous release with unresponsive receiver"]
fn pending_right_release_cancel_is_bounded_and_does_not_replay() {
    let (mut fixture, target, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        SetPropW(
            hwnd,
            w!("Otter.Test.ModalBusy"),
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap()
    };
    let event = |down| Event::Button {
        point: [0.25; 2],
        button: Button::Right,
        down,
        double: false,
    };
    controller.dispatch(1, event(true)).unwrap();
    assert_eq!(
        controller.dispatch_status(2, event(false)),
        Ok(live_input::Delivery::Pending)
    );
    let start = Instant::now();
    controller.cancel();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(!controller.enabled());
    assert!(controller.last_cleanup().failed > 0);
    let records = fixture.finish();
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 1);
}

#[test]
#[ignore = "owned hidden native windows; focus cancellation precedes menu completion callback"]
fn cancellation_collects_menu_completion_within_release_budget() {
    let (mut fixture, target, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    unsafe {
        SetPropW(
            hwnd,
            w!("Otter.Test.ModalNext"),
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap();
    }
    let button = |down| Event::Button {
        point: [0.25; 2],
        button: Button::Right,
        down,
        double: false,
    };
    controller.dispatch(1, button(true)).unwrap();
    assert_eq!(
        controller.dispatch_status(2, button(false)),
        Ok(live_input::Delivery::Pending)
    );
    // 与真实失焦相同：取消开始时菜单仍在嵌套消息泵中，稍后才退出。
    let close = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        unsafe {
            PostMessageW(
                Some(HWND(child as *mut _)),
                CLOSE_MODAL,
                WPARAM(0),
                LPARAM(0),
            )
        }
        .unwrap();
    });
    let start = Instant::now();
    let cleanup = controller.cancel();
    close.join().unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(cleanup.failed, 0);
    controller.enable().unwrap();
    controller.dispatch(3, button(true)).unwrap();
    controller.dispatch(4, button(false)).unwrap();
    assert_eq!(controller.cancel().failed, 0);
    let records = fixture.finish();
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 2);
}
