//! 使用实际 WM_TIMER 队列检查中间矩形，而不是只看滚轮结束后的尺寸。
use super::*;
use std::time::{Duration, Instant};

#[test]
fn geometry_ack_between_wheel_packets_does_not_force_old_frames() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let wp = WPARAM((30usize << 16) | MK_CONTROL.0 as usize);
        let lp = LPARAM(180 | (160 << 16));
        for _ in 0..10 {
            PostMessageW(Some(fixture.window.0), WM_MOUSEWHEEL, wp, lp).unwrap();
            PostMessageW(
                Some(fixture.window.0),
                crate::desktop::window_geometry_wake::MESSAGE,
                WPARAM(0),
                LPARAM(0),
            )
            .unwrap();
        }
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(fixture.window.0),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        assert!(
            !PeekMessageW(
                &mut message,
                Some(fixture.window.0),
                WM_MOUSEWHEEL,
                WM_MOUSEWHEEL,
                PM_NOREMOVE
            )
            .as_bool(),
            "internal geometry ack split a continuous wheel burst"
        );
        animation::settle(fixture.window.0, &fixture.state);
    }
}

#[test]
fn wheel_coalescing_preserves_intervening_messages_and_proxy_input() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        input(&fixture, 30);
        let proxy = preview::handle(&fixture.state).unwrap();
        // 先完成创建/定位产生的宿主通知，再构造无中间业务消息的连续输入 burst。
        let mut pending = MSG::default();
        for _ in 0..64 {
            if !PeekMessageW(&mut pending, Some(fixture.window.0), 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            DispatchMessageW(&pending);
        }
        let before = fixture.rect();
        let wp = WPARAM((30usize << 16) | MK_CONTROL.0 as usize);
        let lp = LPARAM(180 | (160 << 16));
        for _ in 0..10 {
            PostMessageW(Some(proxy), WM_MOUSEWHEEL, wp, lp).unwrap();
        }
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(proxy),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        let mut head = MSG::default();
        if PeekMessageW(&mut head, None, 0, 0, PM_NOREMOVE).as_bool() {
            eprintln!(
                "proxy-burst next_hwnd={:?} next_message=0x{:x}",
                head.hwnd, head.message
            );
        }
        assert!(!PeekMessageW(
            &mut message,
            Some(proxy),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_NOREMOVE
        )
        .as_bool());
        assert!(fixture.rect().width > before.width);
        let before = fixture.rect();
        PostMessageW(Some(proxy), WM_MOUSEWHEEL, wp, lp).unwrap();
        PostMessageW(Some(proxy), WM_APP + 55, WPARAM(0), LPARAM(0)).unwrap();
        PostMessageW(Some(proxy), WM_MOUSEWHEEL, wp, lp).unwrap();
        assert!(PeekMessageW(
            &mut message,
            Some(proxy),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        let after_first = fixture.rect();
        assert!(after_first.width > before.width);
        assert!(PeekMessageW(
            &mut message,
            Some(proxy),
            WM_APP + 55,
            WM_APP + 55,
            PM_REMOVE
        )
        .as_bool());
        assert!(PeekMessageW(
            &mut message,
            Some(proxy),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        assert!(fixture.rect().width > after_first.width);
        animation::settle(fixture.window.0, &fixture.state);
    }
}

#[test]
fn queued_wheel_burst_commits_latest_geometry_without_replaying_old_frames() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let initial = fixture.rect();
        let wp = WPARAM((30usize << 16) | MK_CONTROL.0 as usize);
        let lp = LPARAM(180 | (160 << 16));
        for _ in 0..10 {
            PostMessageW(Some(fixture.window.0), WM_MOUSEWHEEL, wp, lp).unwrap();
        }
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(fixture.window.0),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE
        )
        .as_bool());
        let started = Instant::now();
        DispatchMessageW(&message);
        let elapsed = started.elapsed();
        let mut remaining = 0;
        while PeekMessageW(
            &mut message,
            Some(fixture.window.0),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE,
        )
        .as_bool()
        {
            remaining += 1;
        }
        let expected = crate::pin_geometry::WheelZoom::default()
            .next(initial, (180, 160), 2.5)
            .unwrap();
        eprintln!(
            "wheel-burst first_dispatch_ms={} remaining={} actual={:?} expected={:?}",
            elapsed.as_millis(),
            remaining,
            fixture.rect(),
            expected
        );
        assert_eq!(
            remaining, 0,
            "old wheel messages must not replay after input stops"
        );
        assert_eq!(fixture.rect(), expected);
        animation::settle(fixture.window.0, &fixture.state);
        assert_eq!(fixture.rect(), expected);
    }
}

unsafe fn input(fixture: &Fixture, delta: i16) {
    SendMessageW(
        fixture.window.0,
        WM_MOUSEWHEEL,
        Some(WPARAM(
            ((delta as u16 as usize) << 16) | MK_CONTROL.0 as usize,
        )),
        Some(LPARAM(180 | (160 << 16))),
    );
}
unsafe fn pump(fixture: &Fixture) {
    let mut msg = MSG::default();
    while PeekMessageW(
        &mut msg,
        Some(fixture.window.0),
        WM_TIMER,
        WM_TIMER,
        PM_REMOVE,
    )
    .as_bool()
    {
        DispatchMessageW(&msg);
    }
}

#[test]
fn wheel_reaches_target_before_return_and_idle_timer_never_chases_it() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let initial = fixture.rect();
        input(&fixture, 240);
        let expected = crate::pin_geometry::WheelZoom::default()
            .next(initial, (180, 160), 2.0)
            .unwrap();
        assert_eq!(
            fixture.rect(),
            expected,
            "wheel must commit before returning"
        );
        let started = Instant::now();
        while started.elapsed() < Duration::from_millis(250) {
            pump(&fixture);
            assert_eq!(fixture.rect(), expected, "idle must not animate old input");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(fixture.state.error.borrow().is_none());
    }
}

#[test]
fn rapid_retarget_keeps_full_delta_and_external_move_or_lock_cancels() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let initial = fixture.rect();
        for delta in [120, 120, -120, 30, 30, 30, 30] {
            input(&fixture, delta);
        }
        animation::settle(fixture.window.0, &fixture.state);
        let expected = crate::pin_geometry::WheelZoom::default()
            .next(initial, (180, 160), 2.0)
            .unwrap();
        assert_eq!(fixture.rect(), expected);
        input(&fixture, 120);
        let locked = fixture.rect();
        fixture.state.group_locked.set(true);
        animation::settle(fixture.window.0, &fixture.state);
        assert_eq!(fixture.rect(), locked);
        fixture.state.group_locked.set(false);
        input(&fixture, 120);
        let moved = fixture.rect();
        SetWindowPos(
            fixture.window.0,
            None,
            400,
            300,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        animation::settle(fixture.window.0, &fixture.state);
        assert_eq!((fixture.rect().x, fixture.rect().y), (400, 300));
        assert_eq!(fixture.rect().width, moved.width);
    }
}

#[test]
fn equal_wheel_distance_survives_packet_timing_and_control_key_edges() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for (delta, count, delay) in [(240, 1, 0), (120, 2, 25), (30, 8, 20), (30, 8, 0)] {
            let fixture = Fixture::new();
            let initial = fixture.rect();
            for _ in 0..count {
                input(&fixture, delta);
                std::thread::sleep(Duration::from_millis(delay));
                pump(&fixture);
                // 只发给本 fixture，不注入用户键盘；覆盖通用及左右 Ctrl。
                for key in [0x11, 0xa2, 0xa3] {
                    SendMessageW(fixture.window.0, WM_KEYDOWN, Some(WPARAM(key)), None);
                    SendMessageW(fixture.window.0, WM_KEYUP, Some(WPARAM(key)), None);
                }
            }
            animation::settle(fixture.window.0, &fixture.state);
            let expected = crate::pin_geometry::WheelZoom::default()
                .next(initial, (180, 160), 2.0)
                .unwrap();
            assert_eq!(fixture.rect(), expected, "delta={delta}, delay={delay}");
            assert!(fixture.state.error.borrow().is_none());
        }
    }
}
