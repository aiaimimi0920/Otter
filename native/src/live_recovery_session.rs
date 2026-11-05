//! 恢复子进程会话：先认证再握手，坏流永久封闭，已登记 owner 保留到原父进程退出。
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
    /// pipe 必须是父进程预先创建并交付的 overlapped 客户端端点。
    pub fn accept(expected_parent: u32, pipe: Pipe) -> io::Result<Self> {
        let parent = Parent::authenticate(expected_parent, pipe.handle(), pipe.handle())
            .map_err(io::Error::other)?;
        let exchange = Exchange::accept(pipe, parent.handle(), OPERATION_BUDGET)?;
        Ok(Self { parent, exchange })
    }
    /// true 表示已确认空登记 Shutdown。错误不会删除此前接受的恢复责任。
    pub fn step(&mut self) -> io::Result<bool> {
        self.exchange.step(self.parent.handle(), |record| {
            Binding::bind(record).map_err(io::Error::other)
        })
    }
    pub fn pending(&self) -> usize {
        self.exchange.service.pending()
    }
    /// 仅原父句柄确认退出后恢复；EOF/坏帧不提前改变仍存活父进程的窗口。
    /// 失败记录留在 Session 中，调用者必须保留本 owner 并安排恢复重试。
    pub fn wait_and_recover(&mut self) -> io::Result<()> {
        self.exchange.failed = true;
        match unsafe { WaitForSingleObject(HANDLE(self.parent.handle().as_raw_handle()), INFINITE) }
        {
            WAIT_OBJECT_0 => self
                .exchange
                .service
                .recover(|_, binding| binding.restore_or_retire().map_err(io::Error::other)),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("Unexpected recovery parent wait status")),
        }
    }
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
        // 先标记失败，只有整次请求及 ACK 成功后才重开；半包或写失败绝不重同步猜测边界。
        self.failed = true;
        let mut first = [0u8; 1];
        self.pipe.io(None, Some(peer)).read_exact(&mut first)?;
        let mut channel = self.pipe.io(Some(Instant::now() + self.budget), Some(peer));
        // 空闲首字节无限等待，但收到首字节后的半包和 ACK 共享一个绝对期限。
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
