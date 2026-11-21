use super::*;
use crate::{
    capture_color::SdrFrame,
    capture_dynamic::{Frame, Reason},
    desktop::Window,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
thread_local! { static SHOWN: Cell<usize> = const { Cell::new(0) }; }
unsafe extern "system" fn observe_show(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lp.0 != 0 {
        let message = &*(lp.0 as *const CWPSTRUCT);
        if message.message == WM_SHOWWINDOW
            && message.wParam.0 != 0
            && GetWindowLongPtrW(message.hwnd, GWL_STYLE) as u32 & WS_CHILD.0 == 0
        {
            SHOWN.set(SHOWN.get() + 1);
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
struct ShowProbe(HHOOK);
impl Drop for ShowProbe {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.0).unwrap();
        }
    }
}
#[path = "capture_wait_cached_tests.rs"]
mod cached;
#[path = "capture_wait_completion_tests.rs"]
mod completion;
fn outcome() -> Outcome {
    Outcome {
        frame: Frame::Sdr(SdrFrame {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        }),
        reason: Some(Reason::ExplicitSdr),
        downgraded: false,
        display: None,
        diagnostic: None,
    }
}
unsafe fn hidden<T>(input: &Input<T>) -> Window {
    let template = layout::template();
    let hwnd = CreateDialogIndirectParamW(
        None,
        template.as_ptr().cast(),
        None,
        Some(procedure::<T>),
        LPARAM(input as *const _ as isize),
    )
    .unwrap();
    assert!(!IsWindowVisible(hwnd).as_bool());
    Window(hwnd)
}
unsafe fn settle<T>(hwnd: HWND, input: &Input<T>) {
    let start = Instant::now();
    while input.result.borrow().is_none() {
        SendMessageW(hwnd, WM_TIMER, Some(WPARAM(TIMER)), Some(LPARAM(0)));
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn hidden_dialog_delivers_one_result_and_does_not_restart_completed_job() {
    unsafe {
        let input = Input::new(Box::new(|| CaptureJob::spawn(|_| Ok(outcome()))), false);
        let window = hidden(&input);
        settle(window.0, &input);
        assert!(input.job.borrow().is_none());
        let result = input.result.borrow_mut().take().unwrap().unwrap().unwrap();
        assert_eq!(result.reason, Some(Reason::ExplicitSdr));
        SendMessageW(window.0, WM_TIMER, Some(WPARAM(TIMER)), Some(LPARAM(0)));
        assert!(input.result.borrow().is_none());
        assert!(input.factory.borrow().is_none());
    }
}
#[test]
fn cancel_keeps_dialog_responsive_until_worker_cleanup_and_discards_success() {
    unsafe {
        let (release, wait) = mpsc::sync_channel(1);
        let (ready, entered) = mpsc::sync_channel(1);
        let input = Input::new(
            Box::new(move || {
                CaptureJob::spawn(move |_| {
                    ready.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(3)).unwrap();
                    Ok(outcome())
                })
            }),
            false,
        );
        let window = hidden(&input);
        entered.recv_timeout(Duration::from_secs(3)).unwrap();
        SendMessageW(window.0, WM_COMMAND, Some(WPARAM(2)), Some(LPARAM(0)));
        assert!(input.cancelling.get());
        assert!(input.result.borrow().is_none());
        assert!(input.job.borrow().is_some());
        assert!(!IsWindowEnabled(GetDlgItem(Some(window.0), 2).unwrap()).as_bool());
        release.send(()).unwrap();
        settle(window.0, &input);
        assert!(matches!(input.result.borrow_mut().take(), Some(Ok(None))));
        assert!(input.job.borrow().is_none());
    }
}
#[test]
fn display_change_is_error_and_external_destruction_cancels_and_joins() {
    unsafe {
        let input = Input::new(
            Box::new(|| {
                CaptureJob::spawn(|cancel| {
                    let start = Instant::now();
                    while !cancel.load(std::sync::atomic::Ordering::Acquire) {
                        assert!(start.elapsed() < Duration::from_secs(3));
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Ok(outcome())
                })
            }),
            false,
        );
        let window = hidden(&input);
        SendMessageW(window.0, WM_DISPLAYCHANGE, Some(WPARAM(0)), Some(LPARAM(0)));
        settle(window.0, &input);
        assert!(input
            .result
            .borrow_mut()
            .take()
            .unwrap()
            .err()
            .unwrap()
            .contains("Display layout changed"));
        let input = Input::new(
            Box::new(|| {
                CaptureJob::spawn(|cancel| {
                    let start = Instant::now();
                    while !cancel.load(std::sync::atomic::Ordering::Acquire) {
                        assert!(start.elapsed() < Duration::from_secs(3));
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Ok(outcome())
                })
            }),
            false,
        );
        let window = hidden(&input);
        drop(window);
        assert!(input.job.borrow().is_none());
    }
}

#[test]
fn quick_capture_wait_never_shows_a_dialog_and_returns_the_worker_result() {
    unsafe {
        let _probe = ShowProbe(
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(observe_show),
                None,
                windows::Win32::System::Threading::GetCurrentThreadId(),
            )
            .unwrap(),
        );
        SHOWN.set(0);
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("quick capture owner"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let mut input = Input::new(Box::new(|| CaptureJob::spawn(|_| Ok(outcome()))), false);
        input.quiet = true;
        let result = run(owner.0, input).unwrap().unwrap();
        assert_eq!(result.reason, Some(Reason::ExplicitSdr));
        assert!(!IsWindowVisible(owner.0).as_bool());
        assert_eq!(
            SHOWN.get(),
            0,
            "quick capture must never briefly show its wait dialog"
        );
    }
}

#[test]
fn quiet_wait_escape_still_cancels_and_joins_the_capture_worker() {
    unsafe {
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("quiet cancel owner"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let mut input = Input::new(
            Box::new(|| {
                CaptureJob::spawn(|cancel| {
                    while !cancel.load(std::sync::atomic::Ordering::Acquire) {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Ok(outcome())
                })
            }),
            false,
        );
        input.quiet = true;
        PostMessageW(Some(owner.0), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
        assert!(run(owner.0, input).unwrap().is_none());
    }
}
