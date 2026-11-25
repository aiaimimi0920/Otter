//! Parent-side recovery-process owner; blocking handshakes/requests run only in the background, while UI waits for exit through the original process handle.
use crate::{
    live_recovery_handshake::{read_ready, write_hello},
    live_recovery_pipe::Pipe,
    live_recovery_runtime::FLAG,
    live_recovery_wire::{read_ack, write_request, Command as Operation, Request, Status, SCHEMA},
};
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle},
    process::Command,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::*,
    System::{Com::CoCreateGuid, Threading::*},
};

const BUDGET: Duration = Duration::from_secs(5);
#[path = "live_recovery_async.rs"]
pub mod asynchronous;
#[path = "live_recovery_journal.rs"]
pub mod journal;
#[path = "live_recovery_spawn.rs"]
pub mod spawn;
#[cfg(test)]
use std::{
    os::windows::process::CommandExt,
    process::{Child, Stdio},
};

pub struct Client {
    // Close the pipe before releasing the process handle. Drop does not kill the process: accepted records or records with uncertain ACKs may remain.
    pipe: Pipe,
    child: spawn::Process,
    session: [u8; 16],
    sequence: u64,
    failed: bool,
    shutdown: bool,
}
impl Client {
    pub fn start() -> io::Result<Self> {
        let mut command = Command::new(std::env::current_exe()?);
        command.arg(FLAG).arg(std::process::id().to_string());
        Self::start_command(command)
    }
    fn start_command(command: Command) -> io::Result<Self> {
        let (mut pipe, endpoint) = Pipe::pair()?;
        let session = unsafe { CoCreateGuid() }
            .map_err(io::Error::other)?
            .to_u128()
            .to_le_bytes();
        let mut child = Starting(Some(spawn::spawn(&command, endpoint.into_handle())?));
        // The command carries only paths/arguments/test environment; spawn already closed local temporary inheritable-handle copies.
        drop(command);
        let process = child.0.as_ref().unwrap();
        let mut channel = pipe.io(Some(Instant::now() + BUDGET), Some(process.as_handle()));
        write_hello(&mut channel, session)?;
        read_ready(&mut channel, session)?;
        Ok(Self {
            pipe,
            child: child.0.take().unwrap(),
            session,
            sequence: 1,
            failed: false,
            shutdown: false,
        })
    }
    pub fn process_handle(&self) -> BorrowedHandle<'_> {
        self.child.as_handle()
    }
    pub fn healthy(&self) -> io::Result<bool> {
        if self.failed || self.shutdown {
            return Ok(false);
        }
        match unsafe { WaitForSingleObject(HANDLE(self.child.as_raw_handle()), 0) } {
            WAIT_TIMEOUT => Ok(true),
            WAIT_OBJECT_0 => Ok(false),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("Unexpected recovery child wait status")),
        }
    }
    /// Only an Accepted result permits source-window changes; I/O errors mean uncertain ACKs, not registration rollback.
    /// Failed streams cannot be reused; callers must retain local snapshots and the recovery owner, never directly killing the subprocess.
    pub fn request(&mut self, command: Operation) -> io::Result<Status> {
        if !self.healthy()? {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Recovery child is unavailable",
            ));
        }
        let request = Request {
            schema: SCHEMA,
            session: self.session,
            sequence: self.sequence,
            command,
        };
        request.validate()?;
        let next = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Recovery sequence exhausted"))?;
        self.failed = true;
        let mut channel = self
            .pipe
            .io(Some(Instant::now() + BUDGET), Some(self.child.as_handle()));
        write_request(&mut channel, &request)?;
        let ack = read_ack(&mut channel, self.session, self.sequence)?;
        self.sequence = next;
        self.shutdown =
            matches!(request.command, Operation::Shutdown) && ack.status == Status::Accepted;
        self.failed = false;
        Ok(ack.status)
    }
    /// Wait for normal exit only after an empty-registry Shutdown ACK arrives; do not call during UI message handling.
    pub fn join_shutdown(&mut self) -> io::Result<()> {
        if !self.shutdown {
            return Err(io::Error::other("Recovery shutdown has not been accepted"));
        }
        match unsafe {
            WaitForSingleObject(
                HANDLE(self.child.as_raw_handle()),
                BUDGET.as_millis() as u32,
            )
        } {
            WAIT_OBJECT_0 => {
                if self.child.wait()?.success() {
                    Ok(())
                } else {
                    Err(io::Error::other("Recovery child failed during shutdown"))
                }
            }
            WAIT_TIMEOUT => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Recovery child shutdown timed out",
            )),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other(
                "Unexpected recovery child shutdown wait status",
            )),
        }
    }
}

struct Starting(Option<spawn::Process>);
impl Drop for Starting {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            // No registrations exist before Ready, so this newly created failed subprocess may safely be terminated and reclaimed.
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub fn fixture_command(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "runtime_worker_entries::recovery_client_fixture_entry",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("OTTER_RECOVERY_CLIENT_FIXTURE", mode)
        .env(
            "OTTER_RECOVERY_CLIENT_PARENT",
            std::process::id().to_string(),
        );
    command
}

#[cfg(any(test, feature = "test-support"))]
pub fn run_fixture_child() {
    let mode = std::env::var("OTTER_RECOVERY_CLIENT_FIXTURE").unwrap();
    if mode == "exit" {
        return;
    }
    assert!(matches!(mode.as_str(), "serve" | "shutdown-hang"));
    if let Ok(raw) = std::env::var("OTTER_RECOVERY_INHERIT_EVENT") {
        let raw = raw.parse::<usize>().unwrap();
        let _ = unsafe { SetEvent(HANDLE(raw as *mut _)) };
    }
    let parent = std::env::var("OTTER_RECOVERY_CLIENT_PARENT")
        .unwrap()
        .parse()
        .unwrap();
    crate::live_recovery_runtime::run_inherited(parent).unwrap();
    if mode == "shutdown-hang" {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
#[path = "live_recovery_runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "live_recovery_client_tests.rs"]
mod tests;
