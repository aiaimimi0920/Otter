//! Local overlapped pipes: wait on events for deadlines/peer exit, without read/write threads or idle polling.
use std::{
    io::{self, Read, Write},
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    time::Instant,
};
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::*,
        Storage::FileSystem::{ReadFile, WriteFile},
        System::{Threading::*, IO::*},
    },
};
#[path = "live_recovery_pipe_adopt.rs"]
pub mod adopt;
#[path = "live_recovery_pipe_pair.rs"]
pub mod pair;

pub struct Pipe {
    handle: OwnedHandle,
}
impl Pipe {
    pub fn pair() -> io::Result<(Self, Self)> {
        pair::create()
    }
    /// # Safety
    /// handle must be a pipe created with FILE_FLAG_OVERLAPPED and have no pending operations.
    pub unsafe fn from_overlapped(handle: OwnedHandle) -> Self {
        Self { handle }
    }
    pub fn into_handle(self) -> OwnedHandle {
        self.handle
    }
    pub fn handle(&self) -> BorrowedHandle<'_> {
        self.handle.as_handle()
    }
    /// The deadline covers the entire Channel and is not refreshed by short reads/writes. A timeout may follow a partial transfer;
    /// callers must discard the protocol stream or explicitly retain decoding state, not blindly resend the entire frame.
    /// Cancellation waits for the kernel operation to finish, so no hard real-time bound including cancellation cleanup is promised.
    pub fn io<'a>(
        &'a mut self,
        deadline: Option<Instant>,
        peer: Option<BorrowedHandle<'a>>,
    ) -> Channel<'a> {
        Channel {
            pipe: self,
            deadline,
            peer,
            cancelled: None,
        }
    }
    /// Only explicit callers enable cancellation polling; the recovery-responsibility pipe's default event-wait behavior is unchanged.
    pub fn cancellable_io<'a>(
        &'a mut self,
        deadline: Instant,
        peer: BorrowedHandle<'a>,
        cancelled: &'a dyn Fn() -> bool,
    ) -> Channel<'a> {
        Channel {
            pipe: self,
            deadline: Some(deadline),
            peer: Some(peer),
            cancelled: Some(cancelled),
        }
    }
    fn transfer(
        &mut self,
        deadline: Option<Instant>,
        peer: Option<BorrowedHandle<'_>>,
        cancelled: Option<&dyn Fn() -> bool>,
        start: impl FnOnce(HANDLE, *mut OVERLAPPED) -> windows::core::Result<()>,
    ) -> io::Result<usize> {
        let _ = timeout(deadline)?;
        check_cancel(cancelled)?;
        let event = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
        let event = unsafe { OwnedHandle::from_raw_handle(event.0) };
        let handle = HANDLE(self.handle.as_raw_handle());
        let mut pending = Pending {
            handle,
            overlapped: Box::new(OVERLAPPED {
                hEvent: HANDLE(event.as_raw_handle()),
                ..Default::default()
            }),
            active: false,
        };
        match start(handle, &mut *pending.overlapped) {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_IO_PENDING.0) => {
                pending.active = true;
                let mut handles = [HANDLE(event.as_raw_handle()), HANDLE::default()];
                let count = if let Some(peer) = peer {
                    handles[1] = HANDLE(peer.as_raw_handle());
                    2
                } else {
                    1
                };
                loop {
                    check_cancel(cancelled)?;
                    let remaining = timeout(deadline)?;
                    match unsafe {
                        WaitForMultipleObjects(
                            &handles[..count],
                            false,
                            if cancelled.is_some() {
                                remaining.min(20)
                            } else {
                                remaining
                            },
                        )
                    } {
                        WAIT_OBJECT_0 => break,
                        WAIT_TIMEOUT if cancelled.is_some() => continue,
                        WAIT_TIMEOUT => {
                            return Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "Live recovery pipe deadline exceeded",
                            ))
                        }
                        WAIT_FAILED => return Err(io::Error::last_os_error()),
                        value if value.0 == WAIT_OBJECT_0.0 + 1 && count == 2 => {
                            return Err(io::Error::new(
                                io::ErrorKind::BrokenPipe,
                                "Live recovery peer exited",
                            ))
                        }
                        _ => {
                            return Err(io::Error::other(
                                "Unexpected Live recovery pipe wait status",
                            ))
                        }
                    }
                }
            }
            Err(error) => return Err(io::Error::other(error)),
        }
        let mut transferred = 0;
        unsafe { GetOverlappedResult(handle, &*pending.overlapped, &mut transferred, false) }
            .map_err(io::Error::other)?;
        pending.active = false;
        Ok(transferred as usize)
    }
}
fn check_cancel(cancelled: Option<&dyn Fn() -> bool>) -> io::Result<()> {
    if cancelled.is_some_and(|f| f()) {
        Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Pipe operation cancelled",
        ))
    } else {
        Ok(())
    }
}
struct Pending {
    handle: HANDLE,
    overlapped: Box<OVERLAPPED>,
    active: bool,
}
impl Drop for Pending {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                let _ = CancelIoEx(self.handle, Some(&*self.overlapped));
                let mut count = 0;
                // CancelIoEx only requests cancellation; callers may release/reuse buffers only after completion is confirmed.
                let _ = GetOverlappedResult(self.handle, &*self.overlapped, &mut count, true);
            }
        }
    }
}
fn timeout(deadline: Option<Instant>) -> io::Result<u32> {
    match deadline {
        None => Ok(INFINITE),
        Some(deadline) => {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Live recovery pipe deadline exceeded",
                    )
                })?;
            Ok(remaining
                .as_millis()
                .saturating_add(1)
                .min(u128::from(INFINITE - 1)) as u32)
        }
    }
}
pub struct Channel<'a> {
    pipe: &'a mut Pipe,
    deadline: Option<Instant>,
    peer: Option<BorrowedHandle<'a>>,
    cancelled: Option<&'a dyn Fn() -> bool>,
}
impl Read for Channel<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let count = buffer.len().min(crate::live_recovery_wire::MAX_FRAME_BYTES);
        self.pipe.transfer(
            self.deadline,
            self.peer,
            self.cancelled,
            |handle, overlapped| unsafe {
                ReadFile(handle, Some(&mut buffer[..count]), None, Some(overlapped))
            },
        )
    }
}
impl Write for Channel<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let count = buffer.len().min(crate::live_recovery_wire::MAX_FRAME_BYTES);
        self.pipe.transfer(
            self.deadline,
            self.peer,
            self.cancelled,
            |handle, overlapped| unsafe {
                WriteFile(handle, Some(&buffer[..count]), None, Some(overlapped))
            },
        )
    }
    fn flush(&mut self) -> io::Result<()> {
        // Each write has confirmed transfer completion; do not use FlushFileBuffers to wait for peer consumption and block indefinitely.
        Ok(())
    }
}

#[cfg(test)]
#[path = "live_recovery_pipe_tests.rs"]
mod tests;
