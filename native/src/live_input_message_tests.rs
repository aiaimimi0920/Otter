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

// 窗口完全由夹具创建；没有全局输入、焦点切换或用户窗口枚举。
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
    // 窗口过程返回零，但 SendMessageTimeout 的完成标志必须按成功解释。
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
