//! 正常宿主独占watchdog子进程；不持恢复记录，Drop可安全回收自己的child。
use super::{BUDGET, FLAG};
use crate::{
    live_recovery_client::spawn::{self, Process},
    live_recovery_handshake,
    live_recovery_pipe::Pipe,
};
use std::{
    io,
    os::windows::io::{AsHandle, AsRawHandle},
    process::Command,
    time::Instant,
};
use windows::Win32::{
    Foundation::*,
    System::{Com::CoCreateGuid, Threading::*},
};

pub struct Owner {
    child: Process,
    _pipe: Pipe,
}
impl Owner {
    /// 在产品消息循环之前启动；握手最多原5秒，不在窗口回调中阻塞启动。
    pub fn start() -> io::Result<Self> {
        let mut command = Command::new(std::env::current_exe()?);
        command.arg(FLAG).arg(std::process::id().to_string());
        Self::start_command(command)
    }
    pub(super) fn start_command(command: Command) -> io::Result<Self> {
        let (pipe, endpoint) = Pipe::pair()?;
        let child = spawn::spawn(&command, endpoint.into_handle())?;
        // 即使握手失败也先回收本次自建child；绝不接管现有进程或任意PID。
        let mut owner = Self { child, _pipe: pipe };
        let nonce = unsafe { CoCreateGuid() }
            .map_err(io::Error::other)?
            .to_u128()
            .to_le_bytes();
        let mut channel = owner
            ._pipe
            .io(Some(Instant::now() + BUDGET), Some(owner.child.as_handle()));
        live_recovery_handshake::write_hello(&mut channel, nonce)?;
        live_recovery_handshake::read_ready(&mut channel, nonce)?;
        Ok(owner)
    }
    #[cfg(test)]
    pub(super) fn handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        self.child.as_handle()
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        let handle = HANDLE(self.child.as_raw_handle());
        unsafe {
            if WaitForSingleObject(handle, 0) == WAIT_TIMEOUT {
                let _ = self.child.kill();
            }
            if WaitForSingleObject(handle, BUDGET.as_millis() as u32) != WAIT_OBJECT_0 {
                eprintln!("native-emergency-watchdog: owned child did not exit");
            }
        }
    }
}
