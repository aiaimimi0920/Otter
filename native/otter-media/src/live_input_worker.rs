//! Bounded input workers: at most 16 unconsumed commands, revoke on full capacity, and stop bypasses queued inputs.
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
    /// None is an asynchronous terminal error without a corresponding new command; it does not release command-admission reservations.
    pub command: Option<Command>,
    pub result: Result<crate::live_input::Delivery, Error<E>>,
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
    /// Revocation only; Enable merely wakes recv, and workers check irreversible stop flags before execution.
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
    /// Construct factory/Controller inside workers; calling start alone does not enable input.
    pub fn start<B: Backend<Error = E> + 'static>(
        factory: impl FnOnce() -> Result<B, String> + Send + 'static,
    ) -> io::Result<Self> {
        Self::start_inner(factory, None)
    }
    /// UI threads own workers; replies wake message pumps with WM_NULL, requiring only extra completion-handle waits.
    pub fn start_for_ui<B: Backend<Error = E> + 'static>(
        factory: impl FnOnce() -> Result<B, String> + Send + 'static,
    ) -> io::Result<Self> {
        use windows::Win32::{System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*};
        unsafe {
            // Ensure receiver threads already have message queues without removing existing messages.
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
        // At most 16 ordinary receipts plus one asynchronous-terminal-event slot, without extra input-admission capacity.
        let (results, replies) = mpsc::sync_channel(CAPACITY + 1);
        let signal = stop.clone();
        let event = wake.clone();
        let worker = std::thread::Builder::new()
            .name("otter-live-input".into())
            .spawn(move || {
                let _slot = slot;
                // Controller Drop still attempts cleanup during unwinding; thread handles independently wake terminal-result observation.
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
                // Actual thread handles still expose exit failures; this event does not prove thread reclamation.
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
    /// Full stops the entire control owner, preventing retained pressed states after losing critical edges.
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
    /// Reset before reading to avoid lost concurrent notices; keep signaled after taking an item, resetting on the next empty read.
    pub fn poll_reply(&mut self) -> io::Result<Option<Reply<E>>> {
        // Reset even on an empty queue: a producer may signal after its reply was consumed.
        unsafe { ResetEvent(HANDLE(self.wake.as_raw_handle())) }.map_err(io::Error::other)?;
        match self.replies.try_recv() {
            Ok(reply) => {
                if reply.command.is_some() {
                    self.outstanding = self.outstanding.saturating_sub(1);
                }
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
        // External StopSignal may retain senders, so explicitly wake first; stop flags suppress all queued input.
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
        // Normal UI close retains owners/awaits take_exit; Drop is the final leak safeguard, not guaranteed nonblocking.
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
    loop {
        if stop.load(Ordering::Acquire) {
            break;
        }
        // pending queries may process completion callbacks; sample before checking outcomes and reuse that sample,
        // avoiding a second query clearing tokens before entering endless recv with unreported errors.
        let pending = controller.pending_delivery();
        let (command, result) = if let Err(error) = controller.idle() {
            (None, Err(error))
        } else {
            let received = if pending {
                commands.recv_timeout(std::time::Duration::from_millis(16))
            } else {
                commands
                    .recv()
                    .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
            };
            let command = match received {
                Ok(command) => command,
                Err(mpsc::RecvTimeoutError::Timeout) if !stop.load(Ordering::Acquire) => continue,
                Err(_) => break,
            };
            if stop.load(Ordering::Acquire) {
                break;
            }
            let result = match command {
                Command::Enable => controller
                    .enable()
                    .map(|_| crate::live_input::Delivery::Completed),
                Command::Input { sequence, event } => controller.dispatch_status(sequence, event),
            };
            (Some(command), result)
        };
        let failed = result.is_err();
        let failed_cleanup = controller.last_cleanup();
        let notified = publish(&results, Reply { command, result }, wake, ui_thread);
        if let Err(reason) = notified {
            return Exit {
                reason,
                cleanup: Some(if failed {
                    failed_cleanup
                } else {
                    controller.cancel()
                }),
            };
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
        reason: Reason::Stopped,
        cleanup: Some(controller.cancel()),
    }
}
fn publish<E>(
    results: &mpsc::SyncSender<Reply<E>>,
    reply: Reply<E>,
    wake: &OwnedHandle,
    ui_thread: Option<u32>,
) -> Result<(), Reason> {
    results.try_send(reply).map_err(|_| Reason::WakeFailed)?;
    unsafe { SetEvent(HANDLE(wake.as_raw_handle())) }.map_err(|_| Reason::WakeFailed)?;
    if let Some(thread) = ui_thread {
        use windows::Win32::{
            Foundation::{LPARAM, WPARAM},
            UI::WindowsAndMessaging::{PostThreadMessageW, WM_NULL},
        };
        unsafe { PostThreadMessageW(thread, WM_NULL, WPARAM(0), LPARAM(0)) }
            .map_err(|_| Reason::WakeFailed)?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "live_input_worker_tests.rs"]
mod tests;
