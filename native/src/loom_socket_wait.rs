//! Windows 网络就绪与取消共同等待；取消不依赖复制 socket 的 shutdown 成功。
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
        // WSAEventSelect 将 socket 设为非阻塞。BridgeStream 独占读写，无其它事件 owner。
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
        // 向上取整，避免亚毫秒剩余时间被当作立即超时；保留 INFINITE 值不使用。
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
            // 超长 deadline 可超过 DWORD 的单次等待上限；上限不是新的逻辑截止。
            WAIT_TIMEOUT => Ok(()),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            value if value.0 == WAIT_OBJECT_0.0 + 1 => {
                let mut events = WSANETWORKEVENTS::default();
                // 原子读取并复位，不能单独 ResetEvent 丢掉网络通知。即使收到 CLOSE，
                // 仍让 recv/send 返回剩余数据或具体错误，不提前丢弃缓冲区。
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
