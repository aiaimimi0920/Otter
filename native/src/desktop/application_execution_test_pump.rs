//! 隐藏应用测试使用真实等待器；watchdog 只报告超时，不冒充生产唤醒或周期 poll。
use super::*;
use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows::Win32::System::Threading::GetCurrentThreadId;

const TIMEOUT_MESSAGE: u32 = WM_APP + 997;
const LIMIT: Duration = Duration::from_secs(10);

pub(super) struct Pump {
    deadline: Instant,
    cancel: mpsc::Sender<()>,
    watchdog: Option<JoinHandle<bool>>,
}
impl Pump {
    pub(super) fn new() -> Self {
        let thread_id = unsafe {
            let _ = PeekMessageW(&mut MSG::default(), None, 0, 0, PM_NOREMOVE);
            GetCurrentThreadId()
        };
        let (cancel, waiting) = mpsc::channel();
        let watchdog = thread::spawn(move || {
            if matches!(
                waiting.recv_timeout(LIMIT),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                unsafe {
                    PostThreadMessageW(thread_id, TIMEOUT_MESSAGE, WPARAM(0), LPARAM(0)).unwrap();
                }
                return true;
            }
            false
        });
        Self {
            deadline: Instant::now() + LIMIT,
            cancel,
            watchdog: Some(watchdog),
        }
    }
    unsafe fn next(&self, app: &mut Application) -> (i32, MSG) {
        assert!(
            Instant::now() < self.deadline,
            "application pump deadline exceeded"
        );
        let mut message = MSG::default();
        let status = app.next_live_message(&mut message).unwrap();
        assert_ne!(
            message.message, TIMEOUT_MESSAGE,
            "application wait missed wakeup"
        );
        assert!(
            Instant::now() < self.deadline,
            "application wait exceeded deadline"
        );
        (status, message)
    }
    pub(super) unsafe fn expect_quit(&self, app: &mut Application) {
        loop {
            let (status, message) = self.next(app);
            if status == 0 {
                assert_eq!(message.message, WM_QUIT);
                assert_eq!(message.wParam.0, 0);
                return;
            }
            assert_eq!(status, 1);
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    pub(super) unsafe fn step(&self, app: &mut Application) -> u32 {
        let (status, message) = self.next(app);
        assert_eq!(status, 1);
        if !message.hwnd.is_invalid() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        app.pending_windows(false).unwrap();
        message.message
    }
}
impl Drop for Pump {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
        if let Some(watchdog) = self.watchdog.take() {
            let result = watchdog.join();
            // 不读取或吞掉退出消息；正常收尾不能隐瞒超时或 watchdog 自身失败。
            if !thread::panicking() {
                assert!(
                    !result.expect("application watchdog panicked"),
                    "application pump timed out"
                );
            }
        }
    }
}

#[path = "application_execution_reap_tests.rs"]
mod tests;
