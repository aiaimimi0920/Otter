//! 单个扩展目录订阅 worker；只发布最新不可变快照，失败/取消终止连接，不自动重连。
use super::{
    capability_job::Source,
    loom_execution_signal::{CompletionSignal, Notify},
};
use crate::{
    loom_bridge_stream::Interrupt,
    loom_extension_session::{Error as ExchangeError, Session},
    loom_extension_snapshot::Snapshot,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
struct Permit;
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}
#[derive(Debug)]
pub(super) enum Error {
    Busy,
    Cancelled,
    Discovery(crate::loom_discovery::Error),
    Exchange(ExchangeError),
    Spawn(std::io::Error),
    WorkerFailed,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => f.write_str("extension catalog is busy"),
            Self::Cancelled => f.write_str("extension catalog cancelled"),
            Self::Discovery(error) => write!(f, "extension discovery: {error:?}"),
            Self::Exchange(error) => write!(f, "extension exchange: {error:?}"),
            Self::Spawn(error) => write!(f, "extension worker spawn: {error}"),
            Self::WorkerFailed => f.write_str("extension worker failed"),
        }
    }
}
#[derive(Default)]
struct Mailbox {
    snapshot: Option<Arc<Snapshot>>,
    interrupt: Option<Interrupt>,
}
struct Published(Arc<Mutex<Mailbox>>);
impl Drop for Published {
    fn drop(&mut self) {
        let mut mailbox = self.0.lock().unwrap_or_else(|e| e.into_inner());
        mailbox.snapshot = None;
        mailbox.interrupt = None;
    }
}
struct Running {
    ready: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    worker: JoinHandle<Result<(), Error>>,
}
pub(super) enum Event {
    Snapshot(Arc<Snapshot>),
    Stopped(Result<(), Error>),
}
#[derive(Default)]
pub(super) struct Job {
    running: Option<Running>,
}
impl Job {
    pub(super) fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub(super) fn start(&mut self, source: Source, notify: Notify) -> Result<(), Error> {
        if self.is_busy()
            || ACTIVE
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(Error::Busy);
        }
        let permit = Permit;
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let updates = mailbox.clone();
        let worker = thread::Builder::new()
            .name("otter-extension-catalog".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify.clone());
                let _permit = permit;
                let _published = Published(updates.clone());
                if signal.load(Ordering::Acquire) {
                    return Err(Error::Cancelled);
                }
                let identity = match source {
                    Source::Discovery(config) => config.load().map_err(Error::Discovery)?,
                    Source::Fixed(identity) => identity,
                };
                let mut session = Session::open(&identity, &signal).map_err(Error::Exchange)?;
                let interrupt = session.interrupt().map_err(Error::Exchange)?;
                {
                    let mut mailbox = updates.lock().unwrap_or_else(|e| e.into_inner());
                    if signal.load(Ordering::Acquire) {
                        let _ = interrupt.cancel();
                        return Err(Error::Cancelled);
                    }
                    mailbox.interrupt = Some(interrupt);
                }
                loop {
                    if signal.load(Ordering::Acquire) {
                        return Err(Error::Cancelled);
                    }
                    let snapshot = session
                        .snapshot_handle()
                        .ok_or(Error::Exchange(ExchangeError::Closed))?;
                    updates.lock().unwrap_or_else(|e| e.into_inner()).snapshot = Some(snapshot);
                    // 不跨邮箱锁调用通知；外层 Wake 合并消息，不为每次推送排队一份目录。
                    notify();
                    session.wait_update(&signal).map_err(Error::Exchange)?;
                }
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running {
            ready,
            cancel,
            mailbox,
            worker,
        });
        Ok(())
    }
    pub(super) fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
            let mut mailbox = running.mailbox.lock().unwrap_or_else(|e| e.into_inner());
            mailbox.snapshot = None;
            if let Some(interrupt) = &mailbox.interrupt {
                let _ = interrupt.cancel();
            }
        }
    }
    pub(super) fn poll(&mut self) -> Option<Event> {
        let running = self.running.as_ref()?;
        if running.ready.load(Ordering::Acquire) || running.worker.is_finished() {
            // 终止优先于未读快照；失败连接不得把已排队的旧 authority 再交给 UI。
            return Some(Event::Stopped(self.join()));
        }
        let mut mailbox = running.mailbox.lock().unwrap_or_else(|e| e.into_inner());
        if running.cancel.load(Ordering::Acquire) {
            mailbox.snapshot = None;
            return None;
        }
        mailbox.snapshot.take().map(Event::Snapshot)
    }
    pub(super) fn shutdown(&mut self) {
        self.cancel();
        if self.running.is_some() {
            let _ = self.join();
        }
    }
    fn join(&mut self) -> Result<(), Error> {
        let running = self.running.take().unwrap();
        let result = running.worker.join().unwrap_or(Err(Error::WorkerFailed));
        if running.cancel.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            result
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(test)]
#[path = "extension_catalog_job_tests.rs"]
pub(super) mod tests;
