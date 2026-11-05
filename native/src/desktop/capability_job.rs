//! 单个能力目录后台任务；固定身份随目录返回，取消后不发布结果，不自动重试。
use super::loom_execution_signal::{CompletionSignal, Notify};
use crate::{
    loom_bridge_identity::Identity, loom_discovery, loom_handshake::Handshake,
    loom_handshake_exchange,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
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
pub enum Source {
    Discovery(loom_discovery::Config),
    Fixed(Identity),
}
pub struct Loaded {
    pub identity: Identity,
    pub handshake: Handshake,
}
#[derive(Debug)]
pub enum Error {
    Busy,
    Cancelled,
    Discovery(loom_discovery::Error),
    Exchange(loom_handshake_exchange::Error),
    Spawn(std::io::Error),
    WorkerFailed,
}
struct Running {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: JoinHandle<Result<Loaded, Error>>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
}
impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub fn start(&mut self, source: Source, notify: Notify) -> Result<(), Error> {
        if self.is_busy()
            || ACTIVE
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err(Error::Busy);
        }
        let permit = Permit;
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = thread::Builder::new()
            .name("otter-capabilities".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let _permit = permit;
                if signal.load(Ordering::Acquire) {
                    return Err(Error::Cancelled);
                }
                let identity = match source {
                    Source::Discovery(config) => config.load().map_err(Error::Discovery)?,
                    Source::Fixed(identity) => identity,
                };
                let handshake =
                    loom_handshake_exchange::fetch(&identity, &signal).map_err(Error::Exchange)?;
                Ok(Loaded {
                    identity,
                    handshake,
                })
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running {
            cancel,
            ready,
            worker,
        });
        Ok(())
    }
    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    pub fn poll(&mut self) -> Option<Result<Loaded, Error>> {
        if !self.running.as_ref().is_some_and(|running| {
            running.ready.load(Ordering::Acquire) || running.worker.is_finished()
        }) {
            return None;
        }
        Some(self.join())
    }
    pub fn shutdown(&mut self) {
        self.cancel();
        if self.running.is_some() {
            let _ = self.join();
        }
    }
    fn join(&mut self) -> Result<Loaded, Error> {
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
