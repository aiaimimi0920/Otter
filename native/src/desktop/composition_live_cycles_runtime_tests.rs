//! 连续真实 WGC 生命周期：同会话多次换尺寸、重启输入 owner，跨会话恢复源并回收 child。
use super::*;

unsafe fn redraw(fixture: &Fixture, black: bool) {
    BLACK.set(black);
    for hwnd in [fixture.root, fixture.child] {
        assert!(InvalidateRect(Some(hwnd), None, true).as_bool());
        let _ = UpdateWindow(hwnd);
    }
    windows::Win32::Graphics::Dwm::DwmFlush().unwrap();
}
unsafe fn pixels(
    session: &mut Session,
    owner: &mut Owner,
    size: [u32; 2],
    black: bool,
    after: u64,
) -> u64 {
    until(session, owner, "cycle frame", |session| {
        let Some(frame) = session.window.as_ref().unwrap().frame_for_test() else {
            return false;
        };
        if frame.sequence <= after || [frame.resource.width, frame.resource.height] != size {
            return false;
        }
        let color = pixel(&frame.resource);
        color[3] >= 245
            && color[..3]
                .iter()
                .all(|value| if black { *value <= 10 } else { *value >= 245 })
    });
    session
        .window
        .as_ref()
        .unwrap()
        .frame_for_test()
        .unwrap()
        .sequence
}
unsafe fn click_child(session: &Session, fixture: &Fixture, target: &Target) {
    let hwnd = session.window.as_ref().unwrap().hwnd_for_test();
    let source = target.frame_bounds().unwrap();
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client).unwrap();
    let mut point = POINT { x: 60, y: 35 };
    assert!(ClientToScreen(fixture.child, &mut point).as_bool());
    let x = (point.x - source.left) * client.right / (source.right - source.left);
    let y = (point.y - source.top) * client.bottom / (source.bottom - source.top);
    assert!(x > 0 && x < client.right && y > 0 && y < client.bottom);
    let at = Some(LPARAM((x as u32 | ((y as u32) << 16)) as isize));
    SendMessageW(hwnd, WM_LBUTTONDOWN, None, at);
    SendMessageW(hwnd, WM_LBUTTONUP, None, at);
}
unsafe fn close(session: &mut Session, owner: &mut Owner) {
    session.stop();
    owner.close();
    let end = Instant::now() + Duration::from_secs(12);
    let mut result = None;
    while result.is_none() || !owner.finished() {
        pump();
        owner.poll().unwrap();
        if result.is_none() {
            result = session.poll();
        }
        assert!(Instant::now() < end, "cycle owners did not terminate");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(result, Some(Ok(())));
    assert!(session.window.is_none());
    assert!(session.completion_handle().is_none());
    assert!(session.input_completion_handle().is_none());
}

#[test]
#[ignore = "explicit owned-window WGC repeated lifecycle gate; no global input or activation"]
fn actual_wgc_repeated_resize_input_restart_and_source_restore() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let started = Instant::now();
        for style in [WS_POPUP, WS_OVERLAPPEDWINDOW] {
            for minimized in [false, true] {
                for cycle in 0..3 {
                    let fixture = Fixture::with_style(style);
                    let target = Target::bind(fixture.root).unwrap();
                    // 和 Application 一样先保存可见选区，之后才允许保护 owner 恢复最小化源。
                    let screen = target.frame_bounds().unwrap();
                    if minimized {
                        let _ = ShowWindow(fixture.root, SW_SHOWMINNOACTIVE);
                        windows::Win32::Graphics::Dwm::DwmFlush().unwrap();
                    }
                    let original = Snapshot::read(&target).unwrap();
                    assert_eq!(original.iconic, minimized);
                    let mut owner = Owner::fixture();
                    let protection = owner.acquire(target.clone()).unwrap();
                    let mut session = Session::start_window(target.clone(), screen).unwrap();
                    session.protect(protection);
                    session.show().unwrap();
                    until(
                        &mut session,
                        &mut owner,
                        "cycle initial frame and hidden source",
                        |s| {
                            s.window.as_ref().unwrap().frame_for_test().is_some()
                                && Snapshot::read(&target)
                                    .unwrap()
                                    .layered
                                    .is_some_and(|v| v.alpha == 1)
                        },
                    );
                    let mut sequence = 0;
                    for (resize, [width, height]) in
                        [[240, 160], [340, 220], [280, 180]].into_iter().enumerate()
                    {
                        SetWindowPos(
                            fixture.root,
                            None,
                            0,
                            0,
                            width,
                            height,
                            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
                        )
                        .unwrap();
                        redraw(&fixture, resize % 2 == 1);
                        let bounds = target.frame_bounds().unwrap();
                        let size = [
                            (bounds.right - bounds.left) as u32,
                            (bounds.bottom - bounds.top) as u32,
                        ];
                        sequence =
                            pixels(&mut session, &mut owner, size, resize % 2 == 1, sequence);
                    }
                    for restart in 0..2 {
                        let hwnd = session.window.as_ref().unwrap().hwnd_for_test();
                        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1ff0)), None);
                        until(
                            &mut session,
                            &mut owner,
                            "cycle input enable",
                            Session::input_enabled,
                        );
                        let bounds = target.frame_bounds().unwrap();
                        let size = [
                            (bounds.right - bounds.left) as u32,
                            (bounds.bottom - bounds.top) as u32,
                        ];
                        click_child(&session, &fixture, &target);
                        sequence = pixels(&mut session, &mut owner, size, restart == 0, sequence);
                        let key = 65 + restart;
                        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(key)), None);
                        until(&mut session, &mut owner, "cycle held input", |_| {
                            fixture.received(WM_KEYDOWN, Some(key))
                        });
                        SendMessageW(hwnd, WM_KILLFOCUS, None, None);
                        until(&mut session, &mut owner, "cycle input drained", |s| {
                            !s.input_enabled()
                                && s.input_completion_handle().is_none()
                                && fixture.received(WM_KEYUP, Some(key))
                        });
                        let cleanup = session.input_cleanup().unwrap();
                        assert_eq!(
                            (cleanup.released, cleanup.skipped, cleanup.failed),
                            (1, 0, 0)
                        );
                        assert!(session.take_input_error().is_none());
                    }
                    close(&mut session, &mut owner);
                    let restored = Snapshot::read(&target).unwrap();
                    assert_eq!(restored.extended_style, original.extended_style);
                    assert_eq!(restored.visible, original.visible);
                    // 原合同恢复最小化源时展开；不是重新最小化或沿用测试期的尺寸。
                    assert!(!restored.iconic);
                    assert_eq!(
                        restored.placement.rcNormalPosition,
                        original.placement.rcNormalPosition
                    );
                    println!("native-live-cycle: style={} minimized={minimized} cycle={cycle} resizes=3 input-restarts=2 sequence={sequence} source-restored=true reaped=true", style.0);
                    assert!(
                        started.elapsed() < Duration::from_secs(180),
                        "cycle batch deadline"
                    );
                }
            }
        }
    }
}
