//! UI-owned single-task recovery owner: a worker exists only during operations, with no queue, idle threads, or timed polling.
use super::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

enum Phase {
    Starting(JoinHandle<io::Result<Client>>),
    Idle(Client),
    Running(JoinHandle<(Client, io::Result<Status>)>),
    Closed,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    Started,
    Completed(Status),
}
pub struct Owner {
    phase: Option<Phase>,
}
impl Owner {
    pub fn start() -> io::Result<Self> {
        Self::start_with(Client::start)
    }
    pub(super) fn start_with(
        start: impl FnOnce() -> io::Result<Client> + Send + 'static,
    ) -> io::Result<Self> {
        let task = thread::Builder::new()
            .name("otter-recovery-start".into())
            .spawn(start)?;
        Ok(Self {
            phase: Some(Phase::Starting(task)),
        })
    }
    pub fn busy(&self) -> bool {
        matches!(self.phase, Some(Phase::Starting(_) | Phase::Running(_)))
    }
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        match self.phase.as_ref()? {
            Phase::Starting(task) => Some(task.as_handle()),
            Phase::Running(task) => Some(task.as_handle()),
            _ => None,
        }
    }
    /// Wait directly for the child process while idle; during operations, the worker handles child exit while waiting on the pipe.
    pub fn child_handle(&self) -> Option<BorrowedHandle<'_>> {
        match self.phase.as_ref()? {
            Phase::Idle(client) => Some(client.process_handle()),
            _ => None,
        }
    }
    pub fn healthy(&self) -> io::Result<bool> {
        match self.phase.as_ref() {
            Some(Phase::Idle(client)) => client.healthy(),
            _ => Ok(false),
        }
    }
    pub fn request(&mut self, operation: Operation) -> io::Result<()> {
        self.request_with(move |client| {
            let status = client.request(operation)?;
            // A Shutdown ACK with an empty registry does not mean the process exited. Reuse bounded waiting in the background; on timeout,
            // return to Journal's restored-source reclamation path while UI continues pumping messages and retaining the original child-process handle.
            if client.shutdown {
                client.join_shutdown()?;
            }
            Ok(status)
        })
    }
    fn request_with(
        &mut self,
        request: impl FnOnce(&mut Client) -> io::Result<Status> + Send + 'static,
    ) -> io::Result<()> {
        if !matches!(self.phase, Some(Phase::Idle(_))) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Recovery operation already pending or unavailable",
            ));
        }
        let Some(Phase::Idle(client)) = self.phase.take() else {
            unreachable!()
        };
        // spawn failure drops the closure, so retain another slot owner to keep Client/recovery responsibility from being lost with it.
        let slot = Arc::new(Mutex::new(Some(client)));
        let input = slot.clone();
        match thread::Builder::new()
            .name("otter-recovery-request".into())
            .spawn(move || {
                let mut client = input
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take()
                    .unwrap();
                // Keep Client outside the unwind boundary: unknown ACKs must not lose the original process handle.
                let result = catch_unwind(AssertUnwindSafe(|| request(&mut client)))
                    .unwrap_or_else(|_| {
                        client.failed = true;
                        Err(io::Error::other("Recovery request worker panicked"))
                    });
                (client, result)
            }) {
            Ok(task) => self.phase = Some(Phase::Running(task)),
            Err(error) => {
                self.phase = slot
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take()
                    .map(Phase::Idle);
                return Err(error);
            }
        }
        Ok(())
    }
    /// Called only by Journal after all sources are restored/canceled; a termination request does not mean the process exited.
    pub(super) fn stop_after_restore(&mut self) -> io::Result<()> {
        let Some(Phase::Idle(client)) = self.phase.as_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Recovery worker is not idle",
            ));
        };
        client.failed = true;
        client.child.kill()
    }
    /// Join only after the actual thread exits; deliver each response at most once, without waiting for I/O on UI.
    pub fn poll(&mut self) -> Option<io::Result<Reply>> {
        let ready = match self.phase.as_ref()? {
            Phase::Starting(task) => task.is_finished(),
            Phase::Running(task) => task.is_finished(),
            _ => false,
        };
        if !ready {
            return None;
        }
        let phase = self.phase.take()?;
        self.phase = Some(Phase::Closed);
        Some(match phase {
            Phase::Starting(task) => match task.join() {
                Ok(Ok(client)) => {
                    self.phase = Some(Phase::Idle(client));
                    Ok(Reply::Started)
                }
                Ok(Err(error)) => Err(error),
                Err(_) => Err(io::Error::other("Recovery start worker panicked")),
            },
            Phase::Running(task) => match task.join() {
                Ok((client, result)) => {
                    self.phase = Some(Phase::Idle(client));
                    result.map(Reply::Completed)
                }
                Err(_) => Err(io::Error::other("Recovery request worker panicked")),
            },
            _ => unreachable!(),
        })
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        // The final safeguard does not detach. Normal application shutdown should retain the owner, pump messages, and poll until not busy before Drop.
        // Abnormal Drop may wait for the current I/O deadline/kernel cleanup; this must not be called nonblocking UI shutdown.
        match self.phase.take() {
            Some(Phase::Starting(task)) => {
                let _ = task.join();
            }
            Some(Phase::Running(task)) => {
                let _ = task.join();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "live_recovery_async_tests.rs"]
mod tests;
