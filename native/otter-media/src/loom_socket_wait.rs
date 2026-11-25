//! Joint Windows network-readiness and cancellation wait; cancellation does not depend on successful shutdown of a duplicated socket.
use std::{
    io,
    net::TcpStream,
    os::windows::io::{AsRawHandle, AsRawSocket, FromRawHandle, OwnedHandle},
    sync::Arc,
    time::Instant,
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Networking::WinSock::*,
    System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects},
};

pub(super) struct Wait {
    network: OwnedHandle,
    pub(super) stop: Arc<OwnedHandle>,
    #[cfg(test)]
    pub(super) entered: Arc<std::sync::atomic::AtomicUsize>,
}
fn event() -> io::Result<OwnedHandle> {
    let handle =
        unsafe { CreateEventW(None, true, false, None) }.map_err(|_| io::Error::last_os_error())?;
    Ok(unsafe { OwnedHandle::from_raw_handle(handle.0) })
}
pub(super) fn signal(event: &OwnedHandle) -> io::Result<()> {
    unsafe { SetEvent(HANDLE(event.as_raw_handle())) }.map_err(|_| io::Error::last_os_error())
}
impl Wait {
    pub(super) fn new(socket: &TcpStream) -> io::Result<Self> {
        let wait = Self {
            network: event()?,
            stop: Arc::new(event()?),
            #[cfg(test)]
            entered: Default::default(),
        };
        // WSAEventSelect makes the socket nonblocking. BridgeStream has exclusive read/write access with no other event owner.
        let result = unsafe {
            WSAEventSelect(
                SOCKET(socket.as_raw_socket() as usize),
                Some(WSAEVENT(wait.network.as_raw_handle() as isize)),
                (FD_READ | FD_WRITE | FD_CLOSE) as i32,
            )
        };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() }.0));
        }
        Ok(wait)
    }
    pub(super) fn ready(&self, socket: &TcpStream, deadline: Instant) -> io::Result<()> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Round up so sub-millisecond remaining time is not treated as immediate timeout; leave the reserved INFINITE value unused.
        let millis = remaining
            .as_nanos()
            .div_ceil(1_000_000)
            .min(u128::from(u32::MAX - 1)) as u32;
        let handles = [
            HANDLE(self.stop.as_raw_handle()),
            HANDLE(self.network.as_raw_handle()),
        ];
        #[cfg(test)]
        self.entered
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        match unsafe { WaitForMultipleObjects(&handles, false, millis) } {
            WAIT_OBJECT_0 => Err(io::ErrorKind::ConnectionAborted.into()),
            WAIT_TIMEOUT if Instant::now() >= deadline => Err(io::ErrorKind::TimedOut.into()),
            // A long deadline may exceed DWORD's per-wait limit; that limit is not a new logical deadline.
            WAIT_TIMEOUT => Ok(()),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            value if value.0 == WAIT_OBJECT_0.0 + 1 => {
                let mut events = WSANETWORKEVENTS::default();
                // Read and reset atomically; a separate ResetEvent could lose network notifications. Even after CLOSE,
                // let recv/send return remaining data or the specific error rather than discarding buffers early.
                if unsafe {
                    WSAEnumNetworkEvents(
                        SOCKET(socket.as_raw_socket() as usize),
                        WSAEVENT(self.network.as_raw_handle() as isize),
                        &mut events,
                    )
                } != 0
                {
                    return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() }.0));
                }
                Ok(())
            }
            _ => Err(io::Error::other("Unexpected socket wait result")),
        }
    }
}
