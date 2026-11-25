//! Recovery-child session: authenticate before handshake, permanently close bad streams, and retain registered owners until the original parent exits.
use crate::{
    live_recovery_handshake::{read_hello, write_ready},
    live_recovery_parent::Parent,
    live_recovery_pipe::Pipe,
    live_recovery_service::Service,
    live_recovery_wire::{read_request, write_ack, Record},
    live_window_target::recovery::Binding,
};
use std::{
    io::{self, Read},
    os::windows::io::{AsRawHandle, BorrowedHandle},
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, System::Threading::*};

const OPERATION_BUDGET: Duration = Duration::from_secs(5);

pub struct Session {
    parent: Parent,
    exchange: Exchange<Binding>,
}
impl Session {
    /// pipe must be the overlapped client endpoint created and delivered beforehand by the parent.
    pub fn accept(expected_parent: u32, pipe: Pipe) -> io::Result<Self> {
        let parent = Parent::authenticate(expected_parent, pipe.handle(), pipe.handle())
            .map_err(io::Error::other)?;
        let exchange = Exchange::accept(pipe, parent.handle(), OPERATION_BUDGET)?;
        Ok(Self { parent, exchange })
    }
    /// true means an empty-registry Shutdown was confirmed. Errors do not remove previously accepted recovery responsibilities.
    pub fn step(&mut self) -> io::Result<bool> {
        self.exchange.step(self.parent.handle(), |record| {
            Binding::bind(record).map_err(io::Error::other)
        })
    }
    pub fn pending(&self) -> usize {
        self.exchange.service.pending()
    }
    /// Restore only after the original parent handle confirms exit; EOF/bad frames must not alter windows while the parent is still alive.
    /// Failed records remain in Session; callers must retain this owner and arrange recovery retries.
    pub fn wait_and_recover(&mut self) -> io::Result<()> {
        self.exchange.failed = true;
        match unsafe { WaitForSingleObject(HANDLE(self.parent.handle().as_raw_handle()), INFINITE) }
        {
            WAIT_OBJECT_0 => self.exchange.service.recover(|_, binding| {
                // The helper has no product UI message pump; clear its own completion notifications first, without spanning RefCell borrows.
                binding.drain_helper_notifications();
                binding.restore_or_retire().map_err(restoration_error)
            }),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("Unexpected recovery parent wait status")),
        }
    }
}

fn restoration_error(error: windows::core::Error) -> io::Error {
    let kind = if error.code() == otter_live_hover_hook::native_layer_protocol::PENDING {
        io::ErrorKind::WouldBlock
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(kind, error) // Preserve the original HRESULT; Pending means waiting, not successful restoration.
}

struct Exchange<T> {
    pipe: Pipe,
    service: Service<T>,
    budget: Duration,
    failed: bool,
}
impl<T> Exchange<T> {
    fn accept(mut pipe: Pipe, peer: BorrowedHandle<'_>, budget: Duration) -> io::Result<Self> {
        let mut channel = pipe.io(Some(Instant::now() + budget), Some(peer));
        let session = read_hello(&mut channel)?;
        let service = Service::new(session)?;
        write_ready(&mut channel, session)?;
        Ok(Self {
            pipe,
            service,
            budget,
            failed: false,
        })
    }
    fn step(
        &mut self,
        peer: BorrowedHandle<'_>,
        bind: impl FnOnce(&Record) -> io::Result<T>,
    ) -> io::Result<bool> {
        if self.failed || self.service.shutdown_accepted() {
            return Err(io::Error::other("Live recovery channel is terminal"));
        }
        // Mark failed first, reopening only after the entire request and ACK succeed; partial frames/write failures must never trigger guessed resynchronization.
        self.failed = true;
        let mut first = [0u8; 1];
        self.pipe.io(None, Some(peer)).read_exact(&mut first)?;
        let mut channel = self.pipe.io(Some(Instant::now() + self.budget), Some(peer));
        // Wait indefinitely for the first idle byte, but partial frames and ACK share one absolute deadline after that byte arrives.
        let request =
            read_request(&mut first.as_slice().chain(&mut channel))?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "Missing recovery request")
            })?;
        let ack = self.service.apply(request, bind)?;
        write_ack(&mut channel, &ack)?;
        self.failed = false;
        Ok(self.service.shutdown_accepted())
    }
}

#[cfg(test)]
#[path = "live_recovery_session_tests.rs"]
mod tests;
