use super::*;
#[path = "live_input_busy_motion_tests.rs"]
pub mod busy_motion;
#[path = "live_input_button_runtime_tests.rs"]
pub mod button_runtime;
#[path = "live_input_hover_retirement_tests.rs"]
pub mod hover_retirement;
#[path = "live_input_modal_tests.rs"]
pub mod modal;
#[path = "live_input_pointer_cleanup_tests.rs"]
pub mod pointer_cleanup;
use crate::live_input::{Button, Controller};
use std::{cell::RefCell, sync::mpsc, time::Duration};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::WindowsAndMessaging::*,
    },
};
type Record = (usize, u32, usize, isize);
thread_local! { static TRACE: RefCell<Vec<Record>> = const { RefCell::new(Vec::new()) }; }
unsafe extern "system" fn proc(window: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == windows::Win32::UI::Controls::WM_MOUSELEAVE
        && !RemovePropW(window, w!("Otter.Test.DestroyOnLeave"))
            .unwrap_or_default()
            .0
            .is_null()
    {
        DestroyWindow(window).unwrap();
        return LRESULT(0);
    }
    if let Some(result) = unsafe { modal::handle(window, message, wp, lp) } {
        return result;
    }
    if matches!(
        message,
        WM_MOUSEMOVE | windows::Win32::UI::Controls::WM_MOUSELEAVE
    ) {
        let delay =
            unsafe { RemovePropW(window, w!("Otter.Test.MotionDelay")) }.unwrap_or_default();
        if !delay.0.is_null() {
            std::thread::sleep(Duration::from_millis(delay.0 as usize as u64));
        }
    }
    if matches!(
        message,
        WM_MOUSEMOVE
            | WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_RBUTTONDOWN
            | WM_RBUTTONUP
            | WM_KEYDOWN
            | WM_KEYUP
            | WM_COMMAND
            | WM_MOUSEWHEEL
            | windows::Win32::UI::Controls::WM_MOUSELEAVE
    ) {
        TRACE.with(|trace| {
            trace
                .borrow_mut()
                .push((window.0 as usize, message, wp.0, lp.0))
        });
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(window, message, wp, lp) }
}
struct Fixture {
    thread: u32,
    worker: Option<std::thread::JoinHandle<Vec<Record>>>,
}
impl Fixture {
    fn finish(&mut self) -> Vec<Record> {
        unsafe {
            PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0)).unwrap();
        }
        self.worker.take().unwrap().join().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.worker.is_some() {
            self.finish();
        }
    }
}
fn fixture() -> (Fixture, Target, usize) {
    fixture_with_button_class(false)
}
fn fixture_with_button_class(button: bool) -> (Fixture, Target, usize) {
    fixture_with_class(button, false)
}
fn fixture_with_class(button: bool, browser: bool) -> (Fixture, Target, usize) {
    let (tx, rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || unsafe {
        let instance = GetModuleHandleW(None).unwrap();
        let class = if browser {
            w!("Chrome_WidgetWin_1")
        } else if button {
            w!("WindowsForms10.BUTTON.OtterOwnedFixture")
        } else {
            w!("OtterNativeInputBackendFixture")
        };
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            }),
            0
        );
        let root = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
            class,
            w!(""),
            WS_POPUP,
            100,
            100,
            200,
            200,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .unwrap();
        SetLayeredWindowAttributes(root, COLORREF(0), 1, LWA_ALPHA).unwrap();
        let child = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!(""),
            WS_CHILD | WS_VISIBLE,
            20,
            20,
            100,
            100,
            Some(root),
            None,
            Some(instance.into()),
            None,
        )
        .unwrap();
        let _ = ShowWindow(root, SW_SHOWNOACTIVATE);
        tx.send((
            Target::bind(root).unwrap(),
            child.0 as usize,
            GetCurrentThreadId(),
        ))
        .unwrap();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            DispatchMessageW(&msg);
        }
        DestroyWindow(root).unwrap();
        UnregisterClassW(class, Some(instance.into())).unwrap();
        TRACE.with(|trace| std::mem::take(&mut *trace.borrow_mut()))
    });
    let (root, child, thread) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    (
        Fixture {
            thread,
            worker: Some(worker),
        },
        root,
        child,
    )
}
#[test]
#[ignore = "interactive desktop; owned Chromium-class window without a browser UIA document"]
fn browser_pointer_and_keyboard_do_not_require_scroll_provider() {
    use crate::live_input::Backend as _;
    let (mut fixture, target, child) = fixture_with_class(false, true);
    // 五个同源 Live 超过滚动池的四份容量，但仅点击/键盘不应申请滚动资源。
    let mut backends = Vec::new();
    for _ in 0..5 {
        let mut backend = Backend::new_local_in_test_harness(target.clone(), None).unwrap();
        backend.preflight().unwrap();
        backends.push(backend);
    }
    assert!(backends
        .iter()
        .all(|backend| backend.browser_scroll.is_none()));
    let backend = backends.pop().unwrap();
    let mut controller = Controller::new(backend);
    controller.enable().unwrap();
    for (index, event) in [
        Event::Button {
            point: [0.2; 2],
            button: Button::Left,
            down: true,
            double: false,
        },
        Event::Button {
            point: [0.2; 2],
            button: Button::Left,
            down: false,
            double: false,
        },
        Event::Key {
            code: 65,
            down: true,
        },
        Event::Key {
            code: 65,
            down: false,
        },
    ]
    .into_iter()
    .enumerate()
    {
        controller.dispatch(index as u64 + 1, event).unwrap();
    }
    assert!(controller.enabled());
    assert_eq!(controller.cancel().failed, 0);
    // 首次滚动才启动真实受限 child，之后复用；无 Document 的自有窗口允许原消息路径。
    let mut backend = backends.pop().unwrap();
    let wheel = Event::Wheel {
        point: [0.2; 2],
        delta: -120,
        horizontal: false,
    };
    assert_eq!(backend.send_timeout_ms(wheel), 5750);
    let destination = backend.resolve(wheel, None).unwrap();
    backend
        .send(destination, wheel, 0, backend.send_timeout_ms(wheel))
        .unwrap();
    assert!(backend.browser_scroll.is_some());
    assert_eq!(backend.send_timeout_ms(wheel), 3750);
    let probe = backend
        .browser_scroll
        .as_mut()
        .unwrap()
        .disconnect_for_test();
    let destination = backend.resolve(wheel, None).unwrap();
    assert!(matches!(
        backend.send(destination, wheel, 0, 3750),
        Err(Error::BrowserScroll(_))
    ));
    assert!(backend.browser_scroll.is_none());
    assert!(
        probe.exited(),
        "Failed scroll child must be retired before returning"
    );
    // 新用户输入可建立新流，但失败的滚动不重放。
    assert_eq!(backend.send_timeout_ms(wheel), 5750);
    let destination = backend.resolve(wheel, None).unwrap();
    backend.send(destination, wheel, 0, 5750).unwrap();
    assert!(backend.browser_scroll.is_some());
    assert_eq!(backend.leave().failed, 0);
    drop(backend);
    let trace = fixture.finish();
    for message in [WM_LBUTTONDOWN, WM_LBUTTONUP, WM_KEYDOWN, WM_KEYUP] {
        assert!(
            trace
                .iter()
                .any(|&(hwnd, msg, _, _)| hwnd == child && msg == message),
            "missing {message}: {trace:?}"
        );
    }
}
#[test]
#[ignore = "interactive desktop; replaces only the owned source child"]
fn pointer_survives_navigation_that_destroys_previous_child() {
    let (mut fixture, root, child) = fixture();
    let mut controller = Controller::new(Backend::new(root.clone(), None).unwrap());
    controller.enable().unwrap();
    controller
        .dispatch(1, Event::Move { point: [0.2; 2] })
        .unwrap();
    unsafe {
        let delivered = SendMessageTimeoutW(
            HWND(child as *mut _),
            WM_CLOSE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            1000,
            None,
        );
        assert_ne!(delivered.0, 0);
        assert!(!IsWindow(Some(HWND(child as *mut _))).as_bool());
    }
    let result = controller.dispatch(2, Event::Move { point: [0.2; 2] });
    assert_eq!(
        result,
        Ok(()),
        "stale hover cleanup must not revoke the surviving source"
    );
    assert!(controller.enabled());
    assert_eq!(controller.cancel().failed, 0);
    let trace = fixture.finish();
    assert!(trace
        .iter()
        .any(|&(hwnd, msg, _, _)| hwnd == root.hwnd().0 as usize && msg == WM_MOUSEMOVE));
}
#[test]
#[ignore = "interactive desktop; controller sends only to owned alpha=1 fixture windows"]
fn actual_controller_routes_holds_moves_releases_and_rejects_closed_source() {
    let (mut fixture, root, child) = fixture();
    let mut controller = Controller::new(Backend::new(root, None).unwrap());
    assert_eq!(
        controller.dispatch(
            1,
            Event::Key {
                code: 65,
                down: true
            }
        ),
        Err(live_input::Error::Disabled)
    );
    controller.enable().unwrap();
    controller
        .dispatch(
            1,
            Event::Button {
                point: [0.2; 2],
                button: Button::Left,
                down: true,
                double: false,
            },
        )
        .unwrap();
    controller
        .dispatch(
            2,
            Event::Key {
                code: 65,
                down: true,
            },
        )
        .unwrap();
    // 拖出子控件仍向原目标发送，不重新命中 root。
    controller
        .dispatch(3, Event::Move { point: [0.9; 2] })
        .unwrap();
    let cleanup = controller.cancel();
    assert_eq!(
        cleanup,
        live_input::Cleanup {
            released: 2,
            failed: 0,
            skipped: 0
        }
    );
    controller.enable().unwrap();
    let records = fixture.finish();
    assert!(matches!(
        controller.dispatch(
            4,
            Event::Key {
                code: 65,
                down: true
            }
        ),
        Err(live_input::Error::Backend(_))
    ));
    assert!(!controller.enabled());
    assert_eq!(controller.last_sequence(), 3);
    assert!(records.contains(&(child, WM_LBUTTONDOWN, 1, 20 | (20 << 16))));
    assert!(records.contains(&(child, WM_MOUSEMOVE, 1, 159 | (159 << 16))));
    assert!(records.contains(&(child, WM_LBUTTONUP, 0, 159 | (159 << 16))));
    assert!(records
        .iter()
        .any(|(window, msg, wp, _)| *window == child && *msg == WM_KEYDOWN && *wp == 65));
    assert!(records
        .iter()
        .any(|(window, msg, wp, _)| *window == child && *msg == WM_KEYUP && *wp == 65));
}

#[test]
#[ignore = "interactive desktop; destroys only owned fixture child before cleanup"]
fn actual_cleanup_falls_back_to_root_coordinates_then_skips_destroyed_source() {
    let (mut fixture, root, child) = fixture();
    let root_window = root.hwnd().0 as usize;
    let mut controller = Controller::new(Backend::new(root, None).unwrap());
    controller.enable().unwrap();
    controller
        .dispatch(
            1,
            Event::Button {
                point: [0.2; 2],
                button: Button::Left,
                down: true,
                double: false,
            },
        )
        .unwrap();
    controller
        .dispatch(
            2,
            Event::Key {
                code: 65,
                down: true,
            },
        )
        .unwrap();
    // 仅销毁已知夹具子 HWND，模拟 held 目标在源仍存活时消失。
    let completed = unsafe {
        SendMessageTimeoutW(
            HWND(child as *mut _),
            WM_CLOSE,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            250,
            None,
        )
    };
    assert_ne!(completed.0, 0);
    assert_eq!(
        controller.cancel(),
        live_input::Cleanup {
            released: 2,
            failed: 0,
            skipped: 0
        }
    );
    controller.enable().unwrap();
    controller
        .dispatch(
            3,
            Event::Key {
                code: 66,
                down: true,
            },
        )
        .unwrap();
    let records = fixture.finish();
    assert_eq!(
        controller.cancel(),
        live_input::Cleanup {
            released: 0,
            failed: 0,
            skipped: 1
        }
    );
    assert!(records.contains(&(root_window, WM_LBUTTONUP, 0, 40 | (40 << 16))));
    assert!(records
        .iter()
        .any(|(window, msg, wp, _)| *window == root_window && *msg == WM_KEYUP && *wp == 65));
}

#[test]
#[ignore = "interactive desktop; asynchronous worker controls only owned alpha=1 fixture"]
fn actual_async_worker_dispatches_and_reaps_after_held_input_cleanup() {
    use crate::live_input_worker::{Command, Reason, Worker};
    use std::{os::windows::io::AsRawHandle, time::Instant};
    use windows::Win32::System::Threading::WaitForSingleObject;
    let (mut fixture, root, child) = fixture();
    let mut worker =
        Worker::start(move || Backend::new(root, None).map_err(|error| format!("{error:?}")))
            .unwrap();
    for command in [
        Command::Enable,
        Command::Input {
            sequence: 1,
            event: Event::Button {
                point: [0.2; 2],
                button: Button::Left,
                down: true,
                double: false,
            },
        },
        Command::Input {
            sequence: 2,
            event: Event::Key {
                code: 65,
                down: true,
            },
        },
    ] {
        worker.submit(command).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(reply) = worker.poll_reply().unwrap() {
                reply.result.unwrap();
                break;
            }
            assert!(Instant::now() < deadline);
            unsafe {
                WaitForSingleObject(HANDLE(worker.wake_handle().as_raw_handle()), 100);
            }
        }
    }
    worker.stop();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(worker.completion_handle().unwrap().as_raw_handle()),
                5000,
            )
        },
        WAIT_OBJECT_0
    );
    let exit = worker.take_exit().unwrap();
    assert_eq!(exit.reason, Reason::Stopped);
    assert_eq!(exit.cleanup.unwrap().released, 2);
    assert!(worker.completion_handle().is_none());
    let records = fixture.finish();
    assert!(records.contains(&(child, WM_LBUTTONUP, 0, 20 | (20 << 16))));
    assert!(records
        .iter()
        .any(|(window, msg, wp, _)| *window == child && *msg == WM_KEYUP && *wp == 65));
}
