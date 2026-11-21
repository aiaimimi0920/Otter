//! 自有窗口复现模态按钮；准入和回调完成分开，取消不重发且有界收取回调。
use super::*;
use std::time::Instant;
const CLOSE_MODAL: u32 = WM_APP + 21;
#[path = "live_input_modal_cancel_tests.rs"]
mod cancellation;
#[path = "live_input_modal_idle_tests.rs"]
mod idle;
#[path = "live_input_modal_button_tests.rs"]
mod opening;

pub(super) unsafe fn handle(window: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    if let Some(result) = unsafe { idle::handle(window, message, wp, lp) } {
        return Some(result);
    }
    if message == WM_CANCELMODE
        && !unsafe { GetPropW(window, w!("Otter.Test.ModalCancelMode")) }
            .0
            .is_null()
    {
        TRACE.with(|trace| {
            trace
                .borrow_mut()
                .push((window.0 as usize, message, wp.0, lp.0))
        });
        let _ = unsafe { RemovePropW(window, w!("Otter.Test.ModalActive")) };
        return Some(LRESULT(0));
    }
    if message == CLOSE_MODAL {
        let _ = unsafe { RemovePropW(window, w!("Otter.Test.ModalActive")) };
        return Some(LRESULT(0));
    }
    if message == WM_RBUTTONUP
        && !unsafe { RemovePropW(window, w!("Otter.Test.ModalBusy")) }
            .unwrap_or_default()
            .0
            .is_null()
    {
        TRACE.with(|trace| {
            trace
                .borrow_mut()
                .push((window.0 as usize, message, wp.0, lp.0))
        });
        std::thread::sleep(Duration::from_millis(500));
        return Some(LRESULT(0));
    }
    let configured = unsafe { GetPropW(window, w!("Otter.Test.ModalTrigger")) }.0 as usize;
    let trigger = if configured == 0 {
        WM_RBUTTONUP
    } else {
        configured as u32
    };
    if message != trigger
        || unsafe { RemovePropW(window, w!("Otter.Test.ModalNext")) }
            .unwrap_or_default()
            .0
            .is_null()
    {
        return None;
    }
    TRACE.with(|trace| {
        trace
            .borrow_mut()
            .push((window.0 as usize, message, wp.0, lp.0))
    });
    unsafe {
        SetPropW(
            window,
            w!("Otter.Test.ModalActive"),
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap()
    };
    let end = Instant::now() + Duration::from_secs(3);
    while Instant::now() < end
        && !unsafe { GetPropW(window, w!("Otter.Test.ModalActive")) }
            .0
            .is_null()
    {
        if !unsafe { GetPropW(window, w!("Otter.Test.ModalPumpHold")) }
            .0
            .is_null()
        {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            // 测试断言失败也会析构 fixture；不能吞掉外层线程退出消息。
            if msg.message == WM_QUIT {
                unsafe { PostQuitMessage(msg.wParam.0 as i32) };
                let _ = unsafe { RemovePropW(window, w!("Otter.Test.ModalActive")) };
                return Some(LRESULT(0));
            }
            // 原生菜单的键盘处理发生在嵌套消息泵中；SendMessage 不经过这里。
            if msg.hwnd == window && msg.message == WM_KEYDOWN && msg.wParam.0 == 0x1b {
                TRACE.with(|trace| {
                    trace.borrow_mut().push((
                        window.0 as usize,
                        msg.message,
                        msg.wParam.0,
                        msg.lParam.0,
                    ));
                });
                let remaining =
                    unsafe { GetPropW(window, w!("Otter.Test.ModalEscapeLevels")) }.0 as usize;
                if remaining > 1 {
                    unsafe {
                        SetPropW(
                            window,
                            w!("Otter.Test.ModalEscapeLevels"),
                            Some(HANDLE((remaining - 1) as *mut _)),
                        )
                        .unwrap();
                    }
                    continue;
                }
                let _ = unsafe { RemovePropW(window, w!("Otter.Test.ModalActive")) };
                break;
            }
            unsafe {
                DispatchMessageW(&msg);
            }
            if msg.message == CLOSE_MODAL {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let _ = unsafe { RemovePropW(window, w!("Otter.Test.ModalActive")) };
    Some(LRESULT(0))
}

#[test]
#[ignore = "owned hidden native windows; cancel cannot send Escape up ahead of queued down"]
fn cancel_preserves_escape_queue_order_and_reports_unconfirmed_release() {
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
            w!("Otter.Test.ModalPumpHold"),
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
    controller.dispatch(2, button(false)).unwrap();
    assert_eq!(
        controller.dispatch_status(
            3,
            Event::Key {
                code: 0x1b,
                down: true
            }
        ),
        Ok(live_input::Delivery::Pending)
    );
    let start = Instant::now();
    let cleanup = controller.cancel();
    unsafe {
        RemovePropW(hwnd, w!("Otter.Test.ModalPumpHold")).unwrap();
    }
    let records = fixture.finish();
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(
        cleanup.failed > 0,
        "queued cleanup is not a processed-key acknowledgement"
    );
    let keys: Vec<_> = records
        .iter()
        .filter(|r| r.2 == 0x1b && matches!(r.1, WM_KEYDOWN | WM_KEYUP))
        .map(|r| r.1)
        .collect();
    assert_eq!(keys, [WM_KEYDOWN, WM_KEYUP]);
}

#[test]
#[ignore = "owned hidden native windows; Escape must enter nested menu message queue"]
fn escape_closes_modal_queue_and_allows_next_right_gesture() {
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
    assert_eq!(
        controller.dispatch_status(
            3,
            Event::Key {
                code: 0x1b,
                down: true
            }
        ),
        Ok(live_input::Delivery::Pending)
    );
    let deadline = Instant::now() + Duration::from_secs(1);
    while controller.pending_delivery() && Instant::now() < deadline {
        controller.idle().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        !controller.pending_delivery(),
        "Escape must close the modal queue before its fixture deadline"
    );
    // 菜单已关闭但物理键仍按住时，系统自动重复 down 也不能撤销控制。
    assert_eq!(
        controller.dispatch_status(
            4,
            Event::Key {
                code: 0x1b,
                down: true
            }
        ),
        Ok(live_input::Delivery::IgnoredWhileModal)
    );
    controller
        .dispatch(
            5,
            Event::Key {
                code: 0x1b,
                down: false,
            },
        )
        .unwrap();
    controller.dispatch(6, button(true)).unwrap();
    assert_eq!(
        controller.dispatch_status(7, button(false)),
        Ok(live_input::Delivery::Completed)
    );
    assert!(controller.enabled());
    assert_eq!(controller.cancel().failed, 0);
    let records = fixture.finish();
    assert_eq!(
        records
            .iter()
            .filter(|r| r.1 == WM_KEYDOWN && r.2 == 0x1b)
            .count(),
        1
    );
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 2);
}

#[test]
#[ignore = "owned hidden native windows; modal Escape identity and two-edge queue budget"]
fn modal_escape_rejects_other_targets_and_deduplicates_held_edges() {
    let (mut fixture, root, child) = fixture();
    let hwnd = HWND(child as *mut _);
    let target = Destination::bind(&root, hwnd).unwrap();
    let other = Destination::bind(&root, root.hwnd()).unwrap();
    let mut modal = message::Modal::default();
    unsafe {
        SetPropW(
            hwnd,
            w!("Otter.Test.ModalNext"),
            Some(HANDLE(std::ptr::without_provenance_mut(1))),
        )
        .unwrap();
    }
    let release = Message::button([8, 8], 0, Button::Right, false, false).unwrap();
    message::button_modal(&target, release, &mut modal).unwrap();
    let down = Message::key(0x1b, true).unwrap();
    let up = Message::key(0x1b, false).unwrap();
    assert_eq!(
        modal.post_escape(&other, down),
        Err(message::Error::InvalidPayload)
    );
    assert_eq!(
        modal.post_escape(&target, Message::key(0x41, true).unwrap()),
        Err(message::Error::InvalidPayload)
    );
    assert_eq!(modal.post_escape(&target, down), Ok(true));
    assert_eq!(modal.post_escape(&target, down), Ok(false));
    assert_eq!(modal.post_escape(&target, up), Ok(true));
    assert_eq!(modal.post_escape(&target, up), Ok(false));
    let deadline = Instant::now() + Duration::from_secs(1);
    while modal.pending() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!modal.pending());
    assert_eq!(
        modal.post_escape(&target, down),
        Err(message::Error::InvalidPayload)
    );
    let records = fixture.finish();
    assert_eq!(
        records
            .iter()
            .filter(|r| r.1 == WM_KEYDOWN && r.2 == 0x1b)
            .count(),
        1
    );
    assert_eq!(
        records
            .iter()
            .filter(|r| r.1 == WM_KEYUP && r.2 == 0x1b)
            .count(),
        1
    );
}

#[test]
#[ignore = "owned hidden native windows; normal right release and held-button cleanup"]
fn successful_right_release_and_cancel_keep_exactly_one_release_per_press() {
    let (mut fixture, target, _) = fixture();
    let mut controller = Controller::new(Backend::new_local(target, None).unwrap());
    controller.enable().unwrap();
    let event = |down| Event::Button {
        point: [0.25; 2],
        button: Button::Right,
        down,
        double: false,
    };
    controller.dispatch(1, event(true)).unwrap();
    controller.dispatch(2, event(false)).unwrap();
    assert_eq!(controller.cancel().failed, 0);
    controller.enable().unwrap();
    controller.dispatch(3, event(true)).unwrap();
    let cleanup = controller.cancel();
    assert_eq!(cleanup.released, 1);
    assert_eq!(cleanup.failed, 0);
    let records = fixture.finish();
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONDOWN).count(), 2);
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 2);
}

#[test]
#[ignore = "owned hidden native windows; repeated right gesture consumed without disabling control"]
fn pending_modal_consumes_repeated_right_gesture_without_cleanup_release() {
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
    assert_eq!(
        controller.dispatch_status(3, event(true)),
        Ok(live_input::Delivery::IgnoredWhileModal)
    );
    assert_eq!(
        controller.dispatch_status(4, event(false)),
        Ok(live_input::Delivery::IgnoredWhileModal)
    );
    assert_eq!(
        controller.dispatch_status(5, event(true)),
        Ok(live_input::Delivery::IgnoredWhileModal)
    );
    unsafe {
        PostMessageW(Some(hwnd), CLOSE_MODAL, WPARAM(0), LPARAM(0)).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(1);
    while controller.pending_delivery() && Instant::now() < deadline {
        controller.idle().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!controller.pending_delivery());
    assert_eq!(
        controller.dispatch_status(6, event(false)),
        Ok(live_input::Delivery::IgnoredWhileModal)
    );
    assert!(controller.enabled());
    // 菜单关闭后新手势必须正常投递，不能由消费状态永久屏蔽右键。
    assert_eq!(
        controller.dispatch_status(7, event(true)),
        Ok(live_input::Delivery::Completed)
    );
    assert_eq!(
        controller.dispatch_status(8, event(false)),
        Ok(live_input::Delivery::Completed)
    );
    assert_eq!(controller.cancel().failed, 0);
    let records = fixture.finish();
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONDOWN).count(), 2);
    assert_eq!(records.iter().filter(|r| r.1 == WM_RBUTTONUP).count(), 2);
}
