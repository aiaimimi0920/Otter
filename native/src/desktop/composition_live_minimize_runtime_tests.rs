//! 自有源最小化期间释放输入、保留控制意图，恢复后台渲染后继续点击和滚动。
use super::*;

#[test]
#[ignore = "interactive desktop; owned WGC source minimize/restore; no global input"]
fn source_minimize_releases_input_and_resumes_without_control_failure() {
    let _apartment = Apartment::new().unwrap();
    for cropped in [false, true] {
        unsafe {
            exercise(cropped);
        }
    }
}
unsafe fn exercise(cropped: bool) {
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let fixture = Fixture::new();
        let target = Target::bind(fixture.root).unwrap();
        let original = Snapshot::read(&target).unwrap();
        let mut owner = Owner::fixture();
        let protection = owner.acquire(target.clone()).unwrap();
        let frame = target.frame_bounds().unwrap();
        let selected = if cropped {
            RECT {
                right: frame.left + 120,
                bottom: frame.top + 80,
                ..frame
            }
        } else {
            frame
        };
        let crop = cropped.then(|| {
            crate::live_window_target::region::Crop::from_screen(frame, selected).unwrap()
        });
        let mut session = Session::start_window_region(target.clone(), crop, selected).unwrap();
        session.protect(protection);
        session.show().unwrap();
        until(&mut session, &mut owner, "initial hidden source", |s| {
            s.input_enabled()
                && s.window.as_ref().unwrap().frame_for_test().is_some()
                && Snapshot::read(&target)
                    .unwrap()
                    .layered
                    .is_some_and(|v| v.alpha == 1)
        });
        let window = session.window.as_ref().unwrap().hwnd_for_test();
        let center = LPARAM(50 | (50 << 16));
        for cycle in 0..3 {
            let sequence = session
                .window
                .as_ref()
                .unwrap()
                .frame_for_test()
                .unwrap()
                .sequence;
            INPUT.with(|input| input.borrow_mut().clear());
            SendMessageW(window, WM_LBUTTONDOWN, None, Some(center));
            SendMessageW(window, WM_KEYDOWN, Some(WPARAM(65)), None);
            until(&mut session, &mut owner, "held inputs", |_| {
                fixture.received(WM_KEYDOWN, Some(65))
            });
            let _ = ShowWindow(fixture.root, SW_SHOWMINNOACTIVE);
            assert!(IsIconic(fixture.root).as_bool());
            // 输入 worker 在 UI owner 处理最小化通知前已收到本条；不重放这次动作。
            session
                .send_input(
                    session.last_input_sequence + 1,
                    crate::live_input::Event::Move { point: [0.5; 2] },
                )
                .unwrap();
            let end = Instant::now() + Duration::from_secs(3);
            while session
                .input
                .as_ref()
                .is_some_and(|worker| !worker.stopping())
            {
                pump();
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            until(
                &mut session,
                &mut owner,
                "minimize transition drained",
                |s| s.input.is_none(),
            );
            assert!(
                session.take_input_error().is_none(),
                "minimize is temporary"
            );
            assert!(
                session.take_input_notice().is_none(),
                "must not show control failure"
            );
            assert!(fixture.received(WM_KEYUP, Some(65)));
            assert!(fixture.received(WM_LBUTTONUP, None));
            assert!(
                !IsIconic(fixture.root).as_bool(),
                "hidden source must keep rendering"
            );
            assert_eq!(Snapshot::read(&target).unwrap().layered.unwrap().alpha, 1);
            assert_eq!(session.input_cleanup().unwrap().failed, 0);
            // 确认恢复的不只是 HWND 状态：真实 WGC 必须交付释放鼠标后的新像素。
            let _ = UpdateWindow(fixture.child);
            until(&mut session, &mut owner, "pixels after minimize", |s| {
                let frame = s.window.as_ref().unwrap().frame_for_test().unwrap();
                let color = pixel(&frame.resource);
                frame.sequence > sequence
                    && color[3] >= 245
                    && color[..3].iter().all(|value| {
                        if BLACK.get() {
                            *value <= 10
                        } else {
                            *value >= 245
                        }
                    })
            });
            // 新手势恢复准入，首个滚轮仍送达源，不能要求系统菜单手动重启控制。
            let mut point = POINT { x: 50, y: 50 };
            assert!(ClientToScreen(window, &mut point).as_bool());
            let at = LPARAM(
                (u32::from(point.x as i16 as u16) | (u32::from(point.y as i16 as u16) << 16))
                    as isize,
            );
            SendMessageW(window, WM_MOUSEWHEEL, Some(WPARAM(120 << 16)), Some(at));
            until(&mut session, &mut owner, "wheel after minimize", |s| {
                s.input_enabled() && fixture.received(WM_MOUSEWHEEL, None)
            });
            assert!(session.take_input_error().is_none());
            eprintln!("native-live-minimize: cropped={cropped} cycle={cycle} released=true new-pixels=true resumed=true notice=false");
        }
        // 用户已召回的源不再由隐藏 lease 接管；最小化期间静默等待，手动还原后可继续。
        session.protection.take();
        until(&mut session, &mut owner, "recalled source", |_| {
            Snapshot::read(&target).unwrap().extended_style == original.extended_style
        });
        let _ = ShowWindow(fixture.root, SW_SHOWMINNOACTIVE);
        SendMessageW(window, WM_MOUSEMOVE, None, Some(center));
        until(&mut session, &mut owner, "visible source paused", |s| {
            s.input.is_none()
        });
        assert!(session.take_input_error().is_none());
        assert!(session.take_input_notice().is_none());
        assert!(IsIconic(fixture.root).as_bool());
        let _ = ShowWindow(fixture.root, SW_SHOWNOACTIVATE);
        SendMessageW(window, WM_MOUSEMOVE, None, Some(center));
        until(
            &mut session,
            &mut owner,
            "restored source control",
            Session::input_enabled,
        );
        super::cycles::close(&mut session, &mut owner);
        assert_eq!(
            Snapshot::read(&target).unwrap().extended_style,
            original.extended_style
        );
    }
}
