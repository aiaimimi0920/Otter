//! DComp-only retirement worker; GPU/D2D owners stay on the UI thread until explicit confirmation.
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    time::{Duration, Instant},
};
use windows::{
    core::HRESULT,
    Win32::{Foundation::E_FAIL, Graphics::DirectComposition::*},
};

const MAX_WAITERS: usize = 2;
static WAITERS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
pub(super) fn serial_for_test() -> std::sync::MutexGuard<'static, ()> {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Admission,
    Spawn,
    Apartment,
    Content,
    Root,
    Commit,
    Completion,
    Worker,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pending,
    Complete,
    Failed { stage: Stage, code: HRESULT },
    TimedOut,
}
struct Permit(&'static AtomicUsize);
impl Permit {
    fn acquire(count: &'static AtomicUsize) -> Option<Self> {
        count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_WAITERS).then_some(n + 1)
            })
            .ok()?;
        Some(Self(count))
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Graph {
    device: IDCompositionDevice,
    visual: IDCompositionVisual,
    target: IDCompositionTarget,
}
// Microsoft documents DComp methods as callable from any thread. This wrapper moves only DComp
// references, never HWND access, Rc, a single-threaded D2D factory, a queue, or a GPU owner.
// https://learn.microsoft.com/en-us/windows/win32/directcomp/architecture-and-components
unsafe impl Send for Graph {}
impl Graph {
    fn retire(self) -> Status {
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
        unsafe {
            if let Err(error) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
                return Status::Failed {
                    stage: Stage::Apartment,
                    code: error.code(),
                };
            }
            let result = self.detach();
            drop(self);
            CoUninitialize();
            result
        }
    }
    unsafe fn detach(&self) -> Status {
        if let Err(error) = self.visual.SetContent(None) {
            return Status::Failed {
                stage: Stage::Content,
                code: error.code(),
            };
        }
        if let Err(error) = self.target.SetRoot(None) {
            return Status::Failed {
                stage: Stage::Root,
                code: error.code(),
            };
        }
        if let Err(error) = self.device.Commit() {
            return Status::Failed {
                stage: Stage::Commit,
                code: error.code(),
            };
        }
        match self.device.WaitForCommitCompletion() {
            Ok(()) => Status::Complete,
            Err(error) => Status::Failed {
                stage: Stage::Completion,
                code: error.code(),
            },
        }
    }
}

#[cfg(test)]
#[path = "composition_retirement_tests.rs"]
mod tests;

pub(super) struct Retirement {
    receiver: Receiver<Status>,
    deadline: Instant,
    status: Status,
}
impl Retirement {
    #[cfg(test)]
    pub(super) fn failed_for_test() -> Self {
        let (_sender, receiver) = mpsc::sync_channel(1);
        Self {
            receiver,
            deadline: Instant::now(),
            status: Status::Failed {
                stage: Stage::Commit,
                code: E_FAIL,
            },
        }
    }
    pub(super) fn begin(
        device: &IDCompositionDevice,
        visual: &IDCompositionVisual,
        target: &IDCompositionTarget,
    ) -> Self {
        let graph = Graph {
            device: device.clone(),
            visual: visual.clone(),
            target: target.clone(),
        };
        Self::launch(move || graph.retire(), &WAITERS, Duration::from_secs(2))
    }
    fn launch(
        work: impl FnOnce() -> Status + Send + 'static,
        count: &'static AtomicUsize,
        timeout: Duration,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut result = Self {
            receiver,
            deadline: Instant::now() + timeout,
            status: Status::Pending,
        };
        let Some(permit) = Permit::acquire(count) else {
            result.status = Status::Failed {
                stage: Stage::Admission,
                code: E_FAIL,
            };
            return result;
        };
        // A timed-out wait cannot safely be cancelled. Keep its slot occupied until the worker
        // and COM releases actually finish; never join this potentially stuck thread on UI/Drop.
        if std::thread::Builder::new()
            .name("otter-dcomp-retire".into())
            .spawn(move || {
                let _permit = permit;
                let status = work();
                let _ = sender.send(status);
            })
            .is_err()
        {
            result.status = Status::Failed {
                stage: Stage::Spawn,
                code: E_FAIL,
            };
        }
        result
    }
    pub(super) fn poll(&mut self) -> Status {
        self.poll_at(Instant::now())
    }
    fn poll_at(&mut self, now: Instant) -> Status {
        if self.status != Status::Pending {
            return self.status;
        }
        // Timeout is terminal: a late worker cannot authorize a new HWND/software transaction.
        self.status = if now >= self.deadline {
            Status::TimedOut
        } else {
            match self.receiver.try_recv() {
                Ok(status) => status,
                Err(TryRecvError::Empty) => Status::Pending,
                Err(TryRecvError::Disconnected) => Status::Failed {
                    stage: Stage::Worker,
                    code: E_FAIL,
                },
            }
        };
        self.status
    }
}
