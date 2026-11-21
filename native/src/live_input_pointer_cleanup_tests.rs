//! 自有 HWND 验证悬浮退出不丢失 held-button 清理坐标，以及无 move 的跨目标切换。
use super::*;
use crate::live_input::Backend as _;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;

fn edge(point: [f64; 2], down: bool) -> Event {
    Event::Button {
        point,
        button: Button::Left,
        down,
        double: false,
    }
}

#[test]
#[ignore = "interactive desktop; messages only to owned Chromium-class HWNDs"]
fn browser_exit_motion_is_local_unheld_and_precedes_leave() {
    for local in [false, true] {
        for held in [false, true] {
            let (mut fixture, root, child) = fixture_with_class(false, true);
            let root_hwnd = root.hwnd().0 as usize;
            let backend = if local {
                Backend::new_local(root, None)
            } else {
                Backend::new(root, None)
            }
            .unwrap();
            let mut controller = Controller::new(backend);
            controller.enable().unwrap();
            let event = if held {
                edge([0.2; 2], true)
            } else {
                Event::Move { point: [0.2; 2] }
            };
            controller.dispatch(1, event).unwrap();
            controller.dispatch(2, Event::Leave).unwrap();
            controller.dispatch(3, Event::Leave).unwrap();
            assert_eq!(controller.cancel().failed, 0);
            let records = fixture.finish();
            let exits: Vec<_> = records
                .iter()
                .enumerate()
                .filter(|(_, r)| r.0 == root_hwnd && r.1 == WM_MOUSEMOVE && r.3 as u32 == u32::MAX)
                .collect();
            assert_eq!(exits.len(), usize::from(local && !held), "{records:?}");
            if let Some((index, packet)) = exits.first() {
                assert_eq!(packet.2, 0);
                assert!(records[*index + 1..]
                    .iter()
                    .any(|r| r.0 == child && r.1 == WM_MOUSELEAVE));
            }
            if held {
                let down = records.iter().find(|r| r.1 == WM_LBUTTONDOWN).unwrap();
                let up = records.iter().find(|r| r.1 == WM_LBUTTONUP).unwrap();
                assert_eq!(up.3, down.3);
            }
        }
    }
}

#[test]
#[ignore = "interactive desktop; messages only to owned Chromium-class HWNDs"]
fn browser_cancel_exits_after_button_release_and_only_once() {
    let (mut fixture, root, _) = fixture_with_class(false, true);
    let root_hwnd = root.hwnd().0 as usize;
    let mut controller = Controller::new(Backend::new_local(root, None).unwrap());
    controller.enable().unwrap();
    controller.dispatch(1, edge([0.2; 2], true)).unwrap();
    assert_eq!(controller.cancel().failed, 0);
    assert_eq!(controller.cancel().failed, 0);
    let records = fixture.finish();
    let up = records.iter().position(|r| r.1 == WM_LBUTTONUP).unwrap();
    let exits: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| r.0 == root_hwnd && r.1 == WM_MOUSEMOVE && r.3 as u32 == u32::MAX)
        .collect();
    assert_eq!(exits.len(), 1, "{records:?}");
    assert!(exits[0].0 > up, "exit motion must never extend the drag");
}

#[test]
#[ignore = "interactive desktop; prepares coordinates without sending to the new HWND"]
fn preparing_browser_wheel_does_not_claim_hover_cleanup() {
    for previous_hover in [false, true] {
        let (mut fixture, root, child) = fixture();
        let mut backend = Backend::new_local(root, None).unwrap();
        if previous_hover {
            let event = Event::Move { point: [0.2; 2] };
            let target = backend.resolve(event, None).unwrap();
            backend
                .send(target, event, 0, backend.send_timeout_ms(event))
                .unwrap();
        }
        let wheel = Event::Wheel {
            point: [0.9; 2],
            delta: -120,
            horizontal: false,
        };
        let target = backend.resolve(wheel, None).unwrap();
        // 与 UIA 初始化/查询失败的边界相同：坐标已准备，但未向新目标发送鼠标消息。
        backend.prepare(&target, wheel, 0).unwrap();
        assert_eq!(backend.leave().failed, 0);
        let records = fixture.finish();
        let leaves: Vec<_> = records.iter().filter(|r| r.1 == WM_MOUSELEAVE).collect();
        assert_eq!(leaves.len(), usize::from(previous_hover), "{records:?}");
        if previous_hover {
            assert_eq!(
                leaves[0].0, child,
                "must clean the actual old target, not the prepared new one"
            );
        }
    }
}

#[test]
#[ignore = "interactive desktop; messages only to owned non-Chromium HWNDs"]
fn leave_while_button_held_preserves_cancel_release_coordinates() {
    let (mut fixture, root, child) = fixture();
    let mut controller = Controller::new(Backend::new_local(root, None).unwrap());
    controller.enable().unwrap();
    controller.dispatch(1, edge([0.2; 2], true)).unwrap();
    controller.dispatch(2, Event::Leave).unwrap();
    let cleanup = controller.cancel();
    let records = fixture.finish();
    assert_eq!(
        cleanup,
        live_input::Cleanup {
            released: 1,
            failed: 0,
            skipped: 0
        },
        "leave must preserve held-button cleanup: {records:?}"
    );
    let down = records
        .iter()
        .find(|r| r.0 == child && r.1 == WM_LBUTTONDOWN)
        .unwrap();
    let up = records
        .iter()
        .find(|r| r.0 == child && r.1 == WM_LBUTTONUP)
        .unwrap();
    assert_eq!(
        up.3, down.3,
        "cleanup must use the original mapped coordinates"
    );
    assert_eq!(up.2, 0);
}

#[test]
#[ignore = "interactive desktop; messages only to owned non-Chromium HWNDs"]
fn wheel_or_button_switch_cleans_previous_hover_without_intermediate_move() {
    for next in [
        Event::Wheel {
            point: [0.9; 2],
            delta: -120,
            horizontal: false,
        },
        edge([0.9; 2], true),
    ] {
        let (mut fixture, root, child) = fixture();
        let root_hwnd = root.hwnd().0 as usize;
        let mut controller = Controller::new(Backend::new_local(root, None).unwrap());
        controller.enable().unwrap();
        controller
            .dispatch(1, Event::Move { point: [0.2; 2] })
            .unwrap();
        controller.dispatch(2, next).unwrap();
        assert_eq!(controller.cancel().failed, 0);
        let records = fixture.finish();
        let leave = records
            .iter()
            .position(|r| r.0 == child && r.1 == WM_MOUSELEAVE);
        let incoming = if matches!(next, Event::Wheel { .. }) {
            WM_MOUSEWHEEL
        } else {
            WM_LBUTTONDOWN
        };
        let incoming = records
            .iter()
            .position(|r| r.0 == root_hwnd && r.1 == incoming)
            .unwrap();
        assert!(
            leave.is_some_and(|index| index < incoming),
            "old hover must leave before new target input: {records:?}"
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.0 == child && r.1 == WM_MOUSELEAVE)
                .count(),
            1
        );
    }
}
