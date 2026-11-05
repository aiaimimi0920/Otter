//! 真实 WGC 窗口局部像素与输入闭环；只拥有自建源、Live 和恢复 child，无系统输入注入。
use super::*;
use crate::live_window_target::region::Crop;

unsafe fn repaint(fixture: &Fixture, white: bool) {
    BLACK.set(white);
    for hwnd in [fixture.root, fixture.child] {
        let _ = InvalidateRect(Some(hwnd), None, true);
        let _ = UpdateWindow(hwnd);
    }
}
fn frame(session: &Session) -> Option<std::sync::Arc<crate::live_frames::Frame<Texture>>> {
    session.window.as_ref()?.frame_for_test()
}
unsafe fn wait_pixels(
    session: &mut Session,
    owner: &mut Owner,
    label: &str,
    white: bool,
    after: u64,
) -> u64 {
    until(session, owner, label, |session| {
        frame(session).is_some_and(|frame| {
            let color = pixel(&frame.resource);
            assert_eq!((frame.resource.width, frame.resource.height), (60, 30));
            frame.sequence > after
                && color[3] >= 245
                && color[..3]
                    .iter()
                    .all(|value| if white { *value >= 245 } else { *value <= 10 })
        })
    });
    frame(session).unwrap().sequence
}
unsafe fn finish(session: &mut Session, owner: &mut Owner) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut result = None;
    while result.is_none() {
        pump();
        owner.poll().unwrap();
        result = session.poll();
        assert!(Instant::now() < deadline, "Crop capture did not terminate");
        std::thread::sleep(Duration::from_millis(2));
    }
    owner.close();
    while !owner.finished() {
        pump();
        owner.poll().unwrap();
        assert!(
            Instant::now() < deadline,
            "Crop recovery child did not terminate"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(session.window.is_none());
    assert!(session.completion_handle().is_none());
    assert!(session.input_completion_handle().is_none());
    result.unwrap()
}
#[derive(Clone, Copy, Debug)]
enum End {
    Stop,
    Shrink,
    Close,
}

#[test]
#[ignore = "interactive desktop; real WGC of own patterned windows, input messages and recovery child only"]
fn actual_wgc_crop_move_resize_input_and_terminal_cleanup() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        for style in [WS_POPUP, WS_OVERLAPPEDWINDOW] {
            for end in [End::Stop, End::Shrink, End::Close] {
                let fixture = Fixture::with_style(style);
                PATTERN.set(true);
                repaint(&fixture, false);
                let target = Target::bind(fixture.root).unwrap();
                let original = Snapshot::read(&target).unwrap();
                let mut origin = POINT { x: 30, y: 20 };
                assert!(ClientToScreen(fixture.child, &mut origin).as_bool());
                let selected = RECT {
                    left: origin.x,
                    top: origin.y,
                    right: origin.x + 60,
                    bottom: origin.y + 30,
                };
                let crop = Crop::from_screen(target.frame_bounds().unwrap(), selected).unwrap();
                let mut owner = Owner::fixture();
                let protection = owner.acquire(target.clone()).unwrap();
                let mut session =
                    Session::start_window_region(target.clone(), Some(crop), selected).unwrap();
                session.protect(protection);
                session.show().unwrap();
                let initial = wait_pixels(&mut session, &mut owner, "initial black crop", false, 0);
                until(&mut session, &mut owner, "source hidden", |_| {
                    Snapshot::read(&target)
                        .unwrap()
                        .layered
                        .is_some_and(|layered| layered.alpha == 1)
                });
                SetWindowPos(
                    fixture.root,
                    None,
                    210,
                    110,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                )
                .unwrap();
                repaint(&fixture, true);
                let moved =
                    wait_pixels(&mut session, &mut owner, "moved white crop", true, initial);
                let window = session.window.as_ref().unwrap().hwnd_for_test();
                SendMessageW(window, WM_SYSCOMMAND, Some(WPARAM(0x1ff0)), None);
                until(
                    &mut session,
                    &mut owner,
                    "crop input enabled",
                    Session::input_enabled,
                );
                let mut client = RECT::default();
                GetClientRect(window, &mut client).unwrap();
                assert_eq!((client.right, client.bottom), (60, 30));
                let at = Some(LPARAM((30 | (15 << 16)) as isize));
                SendMessageW(window, WM_LBUTTONDOWN, None, at);
                SendMessageW(window, WM_LBUTTONUP, None, at);
                wait_pixels(
                    &mut session,
                    &mut owner,
                    "input changed local pixels",
                    false,
                    moved,
                );
                let points = POINTS.with(|points| points.borrow().clone());
                assert!(
                    points
                        .iter()
                        .any(|(hwnd, message, point)| *hwnd == fixture.child.0 as usize
                            && *message == WM_LBUTTONUP
                            && *point == [60, 35]),
                    "points={points:?} crop={crop:?} frame={:?}",
                    target.frame_bounds().unwrap()
                );
                let before_resize = frame(&session).unwrap().sequence;
                SetWindowPos(
                    fixture.root,
                    None,
                    0,
                    0,
                    340,
                    220,
                    SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
                )
                .unwrap();
                repaint(&fixture, true);
                wait_pixels(
                    &mut session,
                    &mut owner,
                    "larger source retains crop",
                    true,
                    before_resize,
                );
                SendMessageW(window, WM_KEYDOWN, Some(WPARAM(65)), None);
                until(&mut session, &mut owner, "held crop key", |_| {
                    fixture.received(WM_KEYDOWN, Some(65))
                });
                match end {
                    End::Stop => session.stop(),
                    End::Shrink => SetWindowPos(
                        fixture.root,
                        None,
                        0,
                        0,
                        25,
                        25,
                        SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
                    )
                    .unwrap(),
                    End::Close => DestroyWindow(fixture.root).unwrap(),
                }
                let result = finish(&mut session, &mut owner);
                if matches!(end, End::Stop) {
                    assert_eq!(result, Ok(()));
                } else {
                    assert!(result.is_err(), "{end:?} did not report source failure");
                }
                let cleanup = session.input_cleanup().unwrap();
                assert_eq!(cleanup.failed, 0);
                if matches!(end, End::Close) {
                    assert_eq!((cleanup.released, cleanup.skipped), (0, 1));
                } else {
                    assert_eq!((cleanup.released, cleanup.skipped), (1, 0));
                }
                if !matches!(end, End::Close) {
                    assert!(INPUT.with(|input| input.borrow().iter().any(
                        |(hwnd, msg, key, hidden)| *hwnd == fixture.child.0 as usize
                            && *msg == WM_KEYUP
                            && *key == 65
                            && *hidden
                    )));
                    let restored = Snapshot::read(&target).unwrap();
                    assert_eq!(restored.extended_style, original.extended_style);
                    assert_eq!(restored.visible, original.visible);
                    assert_eq!(restored.rect, original.rect);
                }
                eprintln!("native-live-crop-runtime: style={} end={end:?} crop=60x30 moved=true resized=true input-local=60,35 capture=actual-wgc reaped=true",style.0);
            }
        }
    }
}
