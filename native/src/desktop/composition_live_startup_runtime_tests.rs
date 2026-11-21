//! ???????????? Live ??????????????
use super::*;

#[test]
#[ignore = "interactive desktop; owned normal/maximized WGC sources; no global input"]
fn cropped_live_starts_from_normal_and_maximized_source() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        for maximized in [false, true] {
            for cycle in 0..3 {
                let fixture = Fixture::with_style(WS_OVERLAPPEDWINDOW);
                if maximized {
                    let _ = ShowWindow(fixture.root, SW_MAXIMIZE);
                }
                pump();
                windows::Win32::Graphics::Dwm::DwmFlush().unwrap();
                let target = Target::bind(fixture.root).unwrap();
                let bounds = target.frame_bounds().unwrap();
                let mut point = POINT { x: 30, y: 20 };
                assert!(ClientToScreen(fixture.child, &mut point).as_bool());
                let selected = RECT {
                    left: point.x,
                    top: point.y,
                    right: point.x + 60,
                    bottom: point.y + 30,
                };
                let crop =
                    crate::live_window_target::region::Crop::from_screen(bounds, selected).unwrap();
                let mut owner = Owner::fixture();
                let protection = owner.acquire(target.clone()).unwrap();
                let mut session =
                    Session::start_window_region(target.clone(), Some(crop), selected).unwrap();
                session.protect(protection);
                session.show().unwrap();
                let deadline = Instant::now() + Duration::from_secs(6);
                loop {
                    pump();
                    owner.poll().unwrap();
                    if let Some(result) = session.poll() {
                        panic!("startup maximized={maximized} cycle={cycle} bounds={bounds:?}: {result:?}");
                    }
                    if let Some(frame) = session.window.as_ref().unwrap().frame_for_test() {
                        assert_eq!((frame.resource.width, frame.resource.height), (60, 30));
                        assert!(pixel(&frame.resource)[3] >= 245);
                        break;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "first frame timed out maximized={maximized} bounds={bounds:?}"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
                eprintln!(
                    "native-live-startup: maximized={maximized} cycle={cycle} first-frame=true"
                );
                super::cycles::close(&mut session, &mut owner);
            }
        }
    }
}
