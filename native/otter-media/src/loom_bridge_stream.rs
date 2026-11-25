//! Underlying Loom TLS/WebSocket stream: the total operation deadline is not refreshed by fragmented reads/writes; authentication and discovery are not its responsibility.
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[cfg(windows)]
#[path = "loom_socket_wait.rs"]
pub mod wait;
#[cfg(not(windows))]
use std::net::Shutdown;

pub struct BridgeStream {
    socket: TcpStream,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    // Drop the socket field first, removing its event association before releasing the network event.
    #[cfg(windows)]
    wait: wait::Wait,
}

/// Expose only interruption; do not let the cancellation owner bypass authentication to read/write the raw socket.
pub struct Interrupt {
    #[cfg(not(windows))]
    socket: TcpStream,
    #[cfg(windows)]
    wake: Arc<std::os::windows::io::OwnedHandle>,
    cancelled: Arc<AtomicBool>,
}
impl Interrupt {
    pub fn cancel(&self) -> io::Result<()> {
        // Record irreversible cancellation first, covering the race while the worker has not entered read or is setting the timeout.
        self.cancelled.store(true, Ordering::Release);
        #[cfg(windows)]
        {
            wait::signal(&self.wake)
        }
        #[cfg(not(windows))]
        {
            match self.socket.shutdown(Shutdown::Both) {
                // Remote close, an already-exited read thread, or repeated cancellation can all reach here; cancellation must be idempotent.
                Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
                result => result,
            }
        }
    }
}

impl BridgeStream {
    /// The caller creates the deadline before connecting so connection and handshake share one total budget.
    pub fn new(socket: TcpStream, deadline: Instant) -> io::Result<Self> {
        #[cfg(not(windows))]
        socket.set_nonblocking(false)?;
        socket.set_nodelay(true)?;
        #[cfg(windows)]
        let wait = wait::Wait::new(&socket)?;
        let stream = Self {
            socket,
            deadline,
            cancelled: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            wait,
        };
        stream.before_io()?;
        Ok(stream)
    }

    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

    pub fn interrupt_handle(&self) -> io::Result<Interrupt> {
        Ok(Interrupt {
            #[cfg(not(windows))]
            socket: self.socket.try_clone()?,
            #[cfg(windows)]
            wake: self.wait.stop.clone(),
            cancelled: self.cancelled.clone(),
        })
    }

    /// Set only once per logical operation; do not refresh before each TLS record or WebSocket fragment.
    pub fn operation_deadline(&mut self, timeout: Duration) -> io::Result<()> {
        if timeout.is_zero() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(io::ErrorKind::InvalidInput)?;
        self.deadline = deadline;
        self.before_io()
    }

    fn before_io(&self) -> io::Result<()> {
        self.check_cancelled()?;
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        #[cfg(not(windows))]
        {
            self.socket.set_read_timeout(Some(remaining))?;
            self.socket.set_write_timeout(Some(remaining))?;
        }
        self.check_cancelled()
    }
    fn transfer(
        &mut self,
        mut operation: impl FnMut(&mut TcpStream) -> io::Result<usize>,
    ) -> io::Result<usize> {
        loop {
            self.before_io()?;
            let result = operation(&mut self.socket);
            #[cfg(windows)]
            if matches!(&result, Err(error) if error.kind() == io::ErrorKind::WouldBlock) {
                self.wait.ready(&self.socket, self.deadline)?;
                continue;
            }
            return result;
        }
    }
    fn check_cancelled(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            // read_exact automatically retries Interrupted; cancellation must be a non-retryable terminal state.
            Err(io::ErrorKind::ConnectionAborted.into())
        } else {
            Ok(())
        }
    }
}
impl Read for BridgeStream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.transfer(|socket| socket.read(bytes))
    }
}
impl Write for BridgeStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.transfer(|socket| socket.write(bytes))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.before_io()?;
        self.socket.flush()
    }
}

#[cfg(test)]
#[path = "loom_bridge_stream_tests.rs"]
mod tests;
