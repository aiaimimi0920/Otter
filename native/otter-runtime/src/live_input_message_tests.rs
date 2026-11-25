use super::*;
#[test]
fn mouse_encoding_preserves_signed_points_masks_and_wheel_axis() {
    let motion = Message::motion([-32768, 32767], 7).unwrap();
    assert_eq!(motion.id, WM_MOUSEMOVE);
    assert_eq!(motion.wp.0, 0x13);
    assert_eq!(motion.lp.0 as u32, 0x7fff8000);
    let wheel = Message::wheel([-1, -2], 4, -120, true).unwrap();
    assert_eq!(wheel.id, WM_MOUSEHWHEEL);
    assert_eq!(wheel.wp.0, 0xff880010);
    assert_eq!(wheel.lp.0 as u32, 0xfffeffff);
    assert_eq!(
        Message::button([0, 0], 1, Button::Left, true, true)
            .unwrap()
            .id,
        WM_LBUTTONDBLCLK
    );
    assert_eq!(
        Message::button([0, 0], 0, Button::Middle, false, false)
            .unwrap()
            .id,
        WM_MBUTTONUP
    );
    assert!(Message::motion([32768, 0], 0).is_err());
    assert!(Message::wheel([0, -32769], 0, 120, false).is_err());
    assert!(Message::motion([0, 0], 8).is_err());
    assert!(Message::wheel([0, 0], 0, i32::MIN, false).is_err());
    assert!(Message::button([0, 0], 0, Button::Left, false, true).is_err());
}
#[test]
fn key_encoding_marks_release_and_rejects_invalid_keys() {
    let down = Message::key(65, true).unwrap();
    let up = Message::key(65, false).unwrap();
    assert_eq!(down.id, WM_KEYDOWN);
    assert_eq!(up.id, WM_KEYUP);
    assert_eq!(up.lp.0 as u32 & 0xc0000000, 0xc0000000);
    assert_eq!(down.lp.0 as u32 & 0xc0000000, 0);
    assert_eq!(down.wp.0, 65);
    assert!(Message::key(0, true).is_err());
    assert!(Message::key(255, false).is_err());
}

#[test]
fn navigation_and_divide_preserve_extended_scan_identity_on_both_edges() {
    for key in [
        VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_HOME, VK_END, VK_PRIOR, VK_NEXT, VK_INSERT,
        VK_DELETE, VK_DIVIDE,
    ] {
        let scan = unsafe { MapVirtualKeyW(u32::from(key.0), MAPVK_VK_TO_VSC_EX) };
        // The navigation VK itself determines extended identity; Chinese layouts may provide only the basic numpad scan code.
        for down in [true, false] {
            let packet = Message::key(key.0, down).unwrap();
            let bits = packet.lp.0 as u32;
            assert_eq!(packet.wp.0, usize::from(key.0));
            assert_eq!(bits & 0x01000000, 0x01000000);
            assert_eq!((bits >> 16) & 0xff, scan & 0xff);
            assert_eq!(bits & 0xc0000000, if down { 0 } else { 0xc0000000 });
            assert_eq!(bits & 0xffff, 1);
            assert_eq!(packet.releasing, !down);
        }
    }
}

#[test]
fn ordinary_numpad_and_e1_keys_do_not_gain_an_e0_flag() {
    for code in [
        65,
        VK_TAB.0,
        VK_RETURN.0,
        VK_ESCAPE.0,
        VK_NUMPAD0.0,
        VK_NUMPAD4.0,
        VK_NUMPAD9.0,
        VK_DECIMAL.0,
        VK_PAUSE.0,
        VK_LCONTROL.0,
        VK_RSHIFT.0,
    ] {
        for down in [true, false] {
            let packet = Message::key(code, down).unwrap();
            assert_eq!(packet.lp.0 as u32 & 0x01000000, 0, "code={code}");
        }
    }
}

#[test]
fn explicit_right_control_keeps_its_e0_prefix_without_changing_the_scan_byte() {
    for down in [true, false] {
        let left = Message::key(VK_LCONTROL.0, down).unwrap().lp.0 as u32;
        let right = Message::key(VK_RCONTROL.0, down).unwrap().lp.0 as u32;
        assert_eq!(left & 0x01000000, 0);
        assert_eq!(right & 0x01000000, 0x01000000);
        assert_eq!(left & 0x00ff0000, right & 0x00ff0000);
        assert_eq!(left & 0xc000ffff, right & 0xc000ffff);
    }
}

// Windows are created entirely by the fixture; no global input, focus changes, or user-window enumeration.
#[test]
#[ignore = "requires interactive default desktop; sends only to owned hidden fixture"]
fn actual_zero_result_timeout_same_thread_and_closed_source() {
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};
    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_KEYDOWN {
            if wp.0 == 65 {
                std::thread::sleep(Duration::from_millis(200));
            }
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    }
    struct Fixture {
        thread: u32,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            unsafe {
                let _ = PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0));
            }
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || unsafe {
        let instance = GetModuleHandleW(None).unwrap();
        let class = w!("OtterLiveInputMessageFixture");
        let atom = RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        });
        assert_ne!(atom, 0);
        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!(""),
            WS_POPUP,
            0,
            0,
            16,
            16,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .unwrap();
        let target = Target::bind(window).unwrap();
        assert_eq!(
            send(&target, Message::key(66, true).unwrap()),
            Err(Error::SameThread)
        );
        assert_eq!(
            release(&target, Message::key(66, true).unwrap()),
            Err(Error::InvalidPayload)
        );
        let child = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!(""),
            WS_CHILD,
            0,
            0,
            8,
            8,
            Some(window),
            None,
            Some(instance.into()),
            None,
        )
        .unwrap();
        let destination = Destination::bind(&target, child).unwrap();
        assert_eq!(
            send_to(&destination, Message::key(66, true).unwrap()),
            Err(Error::SameThread)
        );
        tx.send((target, destination, GetCurrentThreadId()))
            .unwrap();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            DispatchMessageW(&msg);
        }
        DestroyWindow(window).unwrap();
        UnregisterClassW(class, Some(instance.into())).unwrap();
    });
    let (target, destination, thread) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let fixture = Fixture {
        thread,
        worker: Some(worker),
    };
    // The window procedure returns zero, but SendMessageTimeout's completion flag must be interpreted as success.
    assert_eq!(send(&target, Message::key(66, true).unwrap()), Ok(()));
    assert_eq!(release(&target, Message::key(66, false).unwrap()), Ok(()));
    assert_eq!(
        send_to(&destination, Message::key(66, true).unwrap()),
        Ok(())
    );
    assert_eq!(
        release_to(&destination, Message::key(66, false).unwrap()),
        Ok(())
    );
    let start = Instant::now();
    assert_eq!(
        deliver(&target, Message::key(65, true).unwrap(), 50),
        Err(Error::DeliveryFailed)
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    drop(fixture);
    assert!(matches!(
        send_to(&destination, Message::key(66, false).unwrap()),
        Err(Error::Destination(_))
    ));
    assert!(matches!(
        send(&target, Message::key(66, false).unwrap()),
        Err(Error::Security(_))
    ));
}

#[test]
fn local_hover_leave_and_committed_utf16_have_distinct_packets() {
    let hover = Message::hover([12, 34], 0).unwrap();
    assert_eq!(hover.id, WM_MOUSEHOVER);
    assert_eq!(hover.lp.0, 12 | (34 << 16));
    let leave = Message::leave();
    assert_eq!(leave.id, WM_MOUSELEAVE);
    assert!(leave.releasing && leave.moving);
    let exit = Message::exit_motion();
    assert_eq!(exit.id, WM_MOUSEMOVE);
    assert_eq!(exit.wp.0, 0);
    assert_eq!(exit.lp.0 as u32, u32::MAX);
    assert!(exit.releasing && exit.moving);
    assert_eq!(Message::character(0x4e2d).unwrap().id, WM_CHAR);
    assert_eq!(Message::character(0xd83d).unwrap().wp.0, 0xd83d);
    assert!(Message::character(0).is_err());
}
