//! Loom TLS/WebSocket 底层流：操作总截止时间不会因分片读写刷新；不负责认证或发现。
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
mod wait;
#[cfg(not(windows))]
use std::net::Shutdown;

pub struct BridgeStream {
    socket: TcpStream,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    // socket 字段先析构，取消其事件关联后才释放 network event。
    #[cfg(windows)]
    wait: wait::Wait,
}

/// 只暴露中断能力，不让取消 owner 绕过认证层读写原始 socket。
pub struct Interrupt {
    #[cfg(not(windows))]
    socket: TcpStream,
    #[cfg(windows)]
    wake: Arc<std::os::windows::io::OwnedHandle>,
    cancelled: Arc<AtomicBool>,
}
impl Interrupt {
    pub fn cancel(&self) -> io::Result<()> {
        // 先记录不可逆取消，覆盖 worker 尚未进入 read/正在设置超时的竞争窗口。
        self.cancelled.store(true, Ordering::Release);
        #[cfg(windows)]
        {
            wait::signal(&self.wake)
        }
        #[cfg(not(windows))]
        {
            match self.socket.shutdown(Shutdown::Both) {
                // 远端关闭、读取线程先退出或重复取消都可能到达这里；取消应当幂等。
                Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
                result => result,
            }
        }
    }
}

impl BridgeStream {
    /// 调用者在连接前创建 deadline，确保连接及握手共享同一个总预算。
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

    pub fn interrupt_handle(&self) -> io::Result<Interrupt> {
        Ok(Interrupt {
            #[cfg(not(windows))]
            socket: self.socket.try_clone()?,
            #[cfg(windows)]
            wake: self.wait.stop.clone(),
            cancelled: self.cancelled.clone(),
        })
    }

    /// 每个逻辑操作只设置一次；不能在每次 TLS record 或 WebSocket 分片前刷新。
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
            // Interrupted 会被 read_exact 自动重试；取消必须是不可重试的终止状态。
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
