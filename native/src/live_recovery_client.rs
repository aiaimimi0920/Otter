//! 父端恢复进程 owner；只能在后台调用阻塞握手/请求，UI 通过原进程句柄等待退出。
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
pub(crate) mod spawn;
#[cfg(test)]
use std::{
    os::windows::process::CommandExt,
    process::{Child, Stdio},
};

pub struct Client {
    // 先关闭管道，再释放进程句柄。Drop 不杀进程：可能仍有已接受或 ACK 不确定的记录。
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
        // 命令仅携带路径/参数/测试环境；spawn 已关闭本地临时可继承句柄副本。
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
    /// 返回 Accepted 才能修改源窗口；I/O 错误表示 ACK 不确定，不代表登记回滚。
    /// 失败流不可重用；调用方须保留本地快照及恢复 owner，不得直接杀掉子进程。
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
    /// 只允许已收到空登记 Shutdown ACK 后等待正常退出，不在 UI 消息处理中调用。
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
            // Ready 前没有任何登记，可安全终止本次自建失败子进程并回收。
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
#[path = "live_recovery_runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "live_recovery_client_tests.rs"]
mod tests;
