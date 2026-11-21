//! 用自有窗口模拟启动播放后的短时 UI 忙；不重放按钮、不改变远程/Chromium 预算。
use super::*;
use crate::live_input::Backend as _;

fn delay_next_motion(child: usize) {
    unsafe {
        SetPropW(
            HWND(child as *mut _),
            w!("Otter.Test.MotionDelay"),
            Some(HANDLE(125usize as *mut _)),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "interactive desktop; only owned hidden non-Chromium fixture"]
fn local_busy_motion_and_leave_complete_without_revoking_or_replaying_click() {
    let (mut fixture, target, child) = fixture();
    let backend = Backend::new_local(target, None).unwrap();
    assert_eq!(
        backend.send_timeout_ms(Event::Move {
            point: [0.25, 0.25]
        }),
        250
    );
    let mut controller = Controller::new(backend);
    controller.enable().unwrap();
    delay_next_motion(child);
    controller
        .dispatch(
            1,
            Event::Move {
                point: [0.25, 0.25],
            },
        )
        .unwrap();
    assert!(controller.enabled());
    for (sequence, down) in [(2, true), (3, false)] {
        controller
            .dispatch(
                sequence,
                Event::Button {
                    point: [0.25, 0.25],
                    button: Button::Left,
                    down,
                    double: false,
                },
            )
            .unwrap();
    }
    delay_next_motion(child);
    assert_eq!(controller.cancel().failed, 0);
    let messages: Vec<_> = fixture
        .finish()
        .into_iter()
        .map(|(_, message, _, _)| message)
        .collect();
    assert_eq!(
        messages,
        [
            WM_MOUSEMOVE,
            WM_LBUTTONDOWN,
            WM_LBUTTONUP,
            windows::Win32::UI::Controls::WM_MOUSELEAVE
        ]
    );
}

#[test]
#[ignore = "interactive desktop; only owned hidden fixtures"]
fn remote_and_chromium_keep_short_motion_budget() {
    for browser in [false, true] {
        let (_fixture, target, _) = fixture_with_class(false, browser);
        let backend = if browser {
            Backend::new_local(target, None).unwrap()
        } else {
            Backend::new(target, None).unwrap()
        };
        for event in [
            Event::Move {
                point: [0.25, 0.25],
            },
            Event::Hover {
                point: [0.25, 0.25],
            },
            Event::Leave,
        ] {
            assert_eq!(backend.send_timeout_ms(event), 50);
        }
        assert_eq!(
            backend.send_timeout_ms(Event::Key {
                code: 65,
                down: true
            }),
            250
        );
    }
}

#[test]
#[ignore = "interactive desktop; owns a hidden non-Chromium source"]
fn local_browser_adapter_is_opt_in_and_non_chromium_is_unchanged() {
    use crate::live_input::Backend as _;
    let (_fixture, target, _) = fixture();
    let ordinary = Backend::new(target.clone(), None).unwrap();
    assert!(!ordinary.local);
    assert!(ordinary.browser_scroll.is_none());
    let mut local = Backend::new_local(target, None).unwrap();
    assert!(local.local);
    assert!(local.browser_scroll.is_none());
    local.preflight().unwrap();
    assert!(local.browser_scroll.is_none());
    assert_eq!(
        ordinary.send_timeout_ms(Event::Wheel {
            point: [0.5, 0.5],
            delta: -120,
            horizontal: false
        }),
        250
    );
    assert_eq!(
        local.send_timeout_ms(Event::Wheel {
            point: [0.5, 0.5],
            delta: -120,
            horizontal: false
        }),
        250
    );
}
