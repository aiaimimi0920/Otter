//! 有界输入 worker：最多 16 个未消费命令，满载撤销，停止不排在输入队列后面。
use crate::live_input::{Backend, Cleanup, Controller, Error, Event};
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
};
use windows::Win32::{
    Foundation::HANDLE,
    System::Threading::{CreateEventW, ResetEvent, SetEvent},
};
pub const CAPACITY: usize = 16;
static ACTIVE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct Slot;
impl Slot {
    fn acquire() -> io::Result<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 16).then_some(count + 1)
            })
            .map_err(|_| {
                io::Error::new(io::ErrorKind::WouldBlock, "Live input worker limit reached")
            })?;
        Ok(Self)
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Enable,
    Input { sequence: u64, event: Event },
}
#[derive(Debug)]
pub struct Reply<E> {
    pub command: Command,
    pub result: Result<(), Error<E>>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Reason {
    Stopped,
    InputFailed,
    Initialization(String),
    Panicked,
    WakeFailed,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Exit {
    pub reason: Reason,
    pub cleanup: Option<Cleanup>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Stopped,
}
#[derive(Clone)]
pub struct StopSignal {
    stop: Arc<AtomicBool>,
    sender: mpsc::SyncSender<Command>,
}
impl StopSignal {
    /// 只撤销；Enable 仅用于唤醒 recv，worker 在执行前检查不可逆 stop 标志。
    pub fn request(&self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.sender.try_send(Command::Enable);
    }
}
pub struct Worker<E> {
    sender: Option<mpsc::SyncSender<Command>>,
    replies: mpsc::Receiver<Reply<E>>,
    stop: Arc<AtomicBool>,
    wake: Arc<OwnedHandle>,
    worker: Option<JoinHandle<Exit>>,
    outstanding: usize,
}
impl<E: Send + 'static> Worker<E> {
    /// factory 和 Controller 都在 worker 内构建；调用 start 本身不启用输入。
    pub fn start<B: Backend<Error = E> + 'static>(
        factory: impl FnOnce() -> Result<B, String> + Send + 'static,
    ) -> io::Result<Self> {
        Self::start_inner(factory, None)
    }
    /// UI 线程拥有此 worker；回复用 WM_NULL 唤醒消息泵，只需额外等待完成句柄。
    pub fn start_for_ui<B: Backend<Error = E> + 'static>(
        factory: impl FnOnce() -> Result<B, String> + Send + 'static,
    ) -> io::Result<Self> {
        use windows::Win32::{System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*};
        unsafe {
            // 保证接收线程已经有消息队列；不移除任何现有消息。
            let _ = PeekMessageW(&mut MSG::default(), None, 0, 0, PM_NOREMOVE);
            Self::start_inner(factory, Some(GetCurrentThreadId()))
        }
    }
    fn start_inner<B: Backend<Error = E> + 'static>(
        factory: impl FnOnce() -> Result<B, String> + Send + 'static,
        ui_thread: Option<u32>,
    ) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let raw = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
        let wake = Arc::new(unsafe { OwnedHandle::from_raw_handle(raw.0) });
        let stop = Arc::new(AtomicBool::new(false));
        let (sender, commands) = mpsc::sync_channel(CAPACITY);
        let (results, replies) = mpsc::sync_channel(CAPACITY);
        let signal = stop.clone();
        let event = wake.clone();
        let worker = std::thread::Builder::new()
            .name("otter-live-input".into())
            .spawn(move || {
                let _slot = slot;
                // Controller 的 Drop 在展开时仍尝试清理；线程句柄是终止结果的独立唤醒源。
                let exit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if signal.load(Ordering::Acquire) {
                        return Exit {
                            reason: Reason::Stopped,
                            cleanup: Some(Cleanup::default()),
                        };
                    }
                    let backend = match factory() {
                        Ok(value) => value,
                        Err(error) => {
                            return Exit {
                                reason: Reason::Initialization(error),
                                cleanup: Some(Cleanup::default()),
                            }
                        }
                    };
                    run(
                        Controller::new(backend),
                        commands,
                        results,
                        &signal,
                        &event,
                        ui_thread,
                    )
                }))
                .unwrap_or(Exit {
                    reason: Reason::Panicked,
                    cleanup: None,
                });
                // 退出失败仍可由真实线程句柄观察；不把此事件当作线程已经回收。
                signal.store(true, Ordering::Release);
                let _ = unsafe { SetEvent(HANDLE(event.as_raw_handle())) };
                exit
            })?;
        Ok(Self {
            sender: Some(sender),
            replies,
            stop,
            wake,
            worker: Some(worker),
            outstanding: 0,
        })
    }
}
impl<E> Worker<E> {
    /// Full 会停止整个控制 owner，避免丢失关键边沿后继续留下按下状态。
    pub fn submit(&mut self, command: Command) -> Result<(), SubmitError> {
        if self.stopping() || self.worker.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err(SubmitError::Stopped);
        }
        if self.outstanding >= CAPACITY {
            self.stop();
            return Err(SubmitError::Full);
        }
        match self
            .sender
            .as_ref()
            .ok_or(SubmitError::Stopped)?
            .try_send(command)
        {
            Ok(()) => {
                self.outstanding += 1;
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.stop();
                Err(SubmitError::Full)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.stop();
                Err(SubmitError::Stopped)
            }
        }
    }
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
    pub fn stop_signal(&self) -> Option<StopSignal> {
        if self.stopping() {
            return None;
        }
        Some(StopSignal {
            stop: self.stop.clone(),
            sender: self.sender.as_ref()?.clone(),
        })
    }
    pub fn outstanding(&self) -> usize {
        self.outstanding
    }
    pub fn wake_handle(&self) -> BorrowedHandle<'_> {
        self.wake.as_handle()
    }
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.worker.as_ref().map(AsHandle::as_handle)
    }
    /// 先复位再读，避免丢失并发通知；取到一项后保持事件，下一次空读再复位。
    pub fn poll_reply(&mut self) -> io::Result<Option<Reply<E>>> {
        unsafe { ResetEvent(HANDLE(self.wake.as_raw_handle())) }.map_err(io::Error::other)?;
        match self.replies.try_recv() {
            Ok(reply) => {
                self.outstanding = self.outstanding.saturating_sub(1);
                unsafe { SetEvent(HANDLE(self.wake.as_raw_handle())) }.map_err(io::Error::other)?;
                Ok(Some(reply))
            }
            Err(_) => Ok(None),
        }
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(Command::Enable);
        }
        // 外部 StopSignal 可能仍持有发送端，因此先显式唤醒；stop 标志抑制所有排队输入。
        self.sender = None;
    }
    pub fn take_exit(&mut self) -> Option<Exit> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        self.stop();
        let exit = self.worker.take()?.join().unwrap_or(Exit {
            reason: Reason::Panicked,
            cleanup: None,
        });
        self.outstanding = 0;
        Some(exit)
    }
}
impl<E> Drop for Worker<E> {
    fn drop(&mut self) {
        self.stop();
        // 常规 UI 关闭须保留 owner 并等待 take_exit；Drop 是最终防泄漏兜底，不保证无阻塞。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn run<B: Backend>(
    mut controller: Controller<B>,
    commands: mpsc::Receiver<Command>,
    results: mpsc::SyncSender<Reply<B::Error>>,
    stop: &AtomicBool,
    wake: &OwnedHandle,
    ui_thread: Option<u32>,
) -> Exit {
    let mut reason = Reason::Stopped;
    while let Ok(command) = commands.recv() {
        if stop.load(Ordering::Acquire) {
            break;
        }
        let result = match command {
            Command::Enable => controller.enable(),
            Command::Input { sequence, event } => controller.dispatch(sequence, event),
        };
        let failed = result.is_err();
        let failed_cleanup = controller.last_cleanup();
        if results.try_send(Reply { command, result }).is_err() {
            break;
        }
        if unsafe { SetEvent(HANDLE(wake.as_raw_handle())) }.is_err() {
            reason = Reason::WakeFailed;
            break;
        }
        if let Some(thread) = ui_thread {
            use windows::Win32::{
                Foundation::{LPARAM, WPARAM},
                UI::WindowsAndMessaging::{PostThreadMessageW, WM_NULL},
            };
            if unsafe { PostThreadMessageW(thread, WM_NULL, WPARAM(0), LPARAM(0)) }.is_err() {
                reason = Reason::WakeFailed;
                break;
            }
        }
        if failed {
            return Exit {
                reason: Reason::InputFailed,
                cleanup: Some(failed_cleanup),
            };
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
    }
    Exit {
        reason,
        cleanup: Some(controller.cancel()),
    }
}
#[cfg(test)]
#[path = "live_input_worker_tests.rs"]
mod tests;
