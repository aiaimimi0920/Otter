//! 网络控制到原UI输入owner的单在途桥；实际worker回复后才确认，不执行Win32输入。
use crate::{live_input::Event, live_input_worker::StopSignal};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::*,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    Enable,
    Input(Event),
    Release,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Busy,
    Deadline,
    Execution,
}
pub(crate) struct Request {
    pub action: Action,
    reply: mpsc::SyncSender<Result<(), Error>>,
}
impl Request {
    pub fn finish(self, result: Result<(), Error>) {
        let _ = self.reply.try_send(result);
    }
}
pub(crate) struct Bridge {
    thread: u32,
    stop: AtomicBool,
    busy: AtomicBool,
    request: Mutex<Option<Request>>,
    input: Mutex<Option<StopSignal>>,
}
impl Bridge {
    /// 仅当前Application线程构造；WM_NULL只唤醒既有泵，不新增等待句柄。
    pub fn new() -> Arc<Self> {
        unsafe {
            let _ = PeekMessageW(&mut MSG::default(), None, 0, 0, PM_NOREMOVE);
        }
        Arc::new(Self {
            thread: unsafe { GetCurrentThreadId() },
            stop: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            request: Mutex::new(None),
            input: Mutex::new(None),
        })
    }
    fn wake(&self) -> Result<(), Error> {
        unsafe { PostThreadMessageW(self.thread, WM_NULL, WPARAM(0), LPARAM(0)) }
            .map_err(|_| Error::Stopped)
    }
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
        self.request
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(input) = self.input.lock().unwrap_or_else(|e| e.into_inner()).take() {
            input.request();
        }
        let _ = self.wake();
    }
    pub fn bind_input(&self, input: Option<StopSignal>) {
        let mut owner = self.input.lock().unwrap_or_else(|e| e.into_inner());
        if self.stopping() {
            if let Some(input) = input {
                input.request();
            }
        } else {
            *owner = input;
        }
    }
    pub fn take(&self) -> Option<Request> {
        if self.stopping() {
            return None;
        }
        self.request
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
    pub fn dispatch(&self, action: Action, check: impl Fn() -> bool) -> Result<(), Error> {
        if self.busy.swap(true, Ordering::AcqRel) {
            return Err(Error::Busy);
        }
        struct Busy<'a>(&'a AtomicBool);
        impl Drop for Busy<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _busy = Busy(&self.busy);
        let (reply, wait) = mpsc::sync_channel(1);
        {
            let mut slot = self.request.lock().unwrap_or_else(|e| e.into_inner());
            if self.stopping() || !check() {
                return Err(Error::Stopped);
            }
            if slot.is_some() {
                return Err(Error::Busy);
            }
            *slot = Some(Request { action, reply });
        }
        if let Err(error) = self.wake() {
            self.cancel();
            return Err(error);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if self.stopping() || !check() {
                self.cancel();
                return Err(Error::Stopped);
            }
            match wait.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(Error::Stopped),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if std::time::Instant::now() >= deadline {
                self.cancel();
                return Err(Error::Deadline);
            }
        }
    }
}
#[cfg(test)]
#[path = "live_relay_source_input_tests.rs"]
mod tests;
