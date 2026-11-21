//! 经过 Windows 输入路由的滚轮回归；仅在自有 Live 位于指针下时注入一次滚轮。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

struct Pointer(POINT);
impl Drop for Pointer {
    fn drop(&mut self) {
        unsafe {
            let _ = SetCursorPos(self.0.x, self.0.y);
        }
    }
}

#[test]
#[ignore = "interactive desktop; sends one wheel only after verifying owned hit window"]
fn actual_windows_wheel_enters_live_and_reaches_hidden_source() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _dpi = Dpi(old);
        let fixture = Fixture::new();
        let target = Target::bind(fixture.root).unwrap();
        let mut owner = Owner::fixture();
        let protection = owner.acquire(target.clone()).unwrap();
        let mut session =
            Session::start_window(target.clone(), target.frame_bounds().unwrap()).unwrap();
        session.protect(protection);
        session.show().unwrap();
        let hwnd = session.window.as_ref().unwrap().hwnd_for_test();
        until(&mut session, &mut owner, "first frame", |session| {
            session.window.as_ref().unwrap().frame_for_test().is_some() && session.input_enabled()
        });
        let mut prior = POINT::default();
        GetCursorPos(&mut prior).unwrap();
        let _pointer = Pointer(prior);
        SetWindowPos(hwnd, Some(HWND_TOPMOST), 240, 240, 200, 100, SWP_NOACTIVATE).unwrap();
        let mut point = POINT { x: 100, y: 50 };
        assert!(ClientToScreen(hwnd, &mut point).as_bool());
        SetCursorPos(point.x, point.y).unwrap();
        pump();
        assert_eq!(WindowFromPoint(point), hwnd);
        let routed = session.window.as_ref().unwrap().wheel_routed_for_test();
        assert!([VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
            .iter()
            .all(|key| GetAsyncKeyState(key.0 as i32) >= 0));
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: (((point.x - GetSystemMetrics(SM_XVIRTUALSCREEN)) as i64 * 65536 + 32768)
                        / GetSystemMetrics(SM_CXVIRTUALSCREEN) as i64)
                        as i32,
                    dy: (((point.y - GetSystemMetrics(SM_YVIRTUALSCREEN)) as i64 * 65536 + 32768)
                        / GetSystemMetrics(SM_CYVIRTUALSCREEN) as i64)
                        as i32,
                    mouseData: 120,
                    dwFlags: MOUSEEVENTF_WHEEL
                        | MOUSEEVENTF_MOVE
                        | MOUSEEVENTF_ABSOLUTE
                        | MOUSEEVENTF_VIRTUALDESK,
                    ..Default::default()
                },
            },
        };
        assert_eq!(SendInput(&[input], std::mem::size_of::<INPUT>() as i32), 1);
        until(
            &mut session,
            &mut owner,
            "physical wheel source receipt",
            |session| {
                fixture.received(WM_MOUSEWHEEL, None)
                    && session.window.as_ref().unwrap().wheel_routed_for_test() > routed
            },
        );
        session.stop();
        owner.close();
        let deadline = Instant::now() + Duration::from_secs(12);
        let mut finished = false;
        while !finished || !owner.finished() {
            pump();
            owner.poll().unwrap();
            if !finished {
                finished = session.poll().is_some_and(|result| result.is_ok());
            }
            assert!(Instant::now() < deadline, "wheel fixture cleanup");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
