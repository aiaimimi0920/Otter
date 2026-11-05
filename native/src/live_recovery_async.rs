//! UI 持有的单任务恢复 owner：仅操作期间存在 worker，无队列、无空闲线程或定时轮询。
use super::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

enum Phase {
    Starting(JoinHandle<io::Result<Client>>),
    Idle(Client),
    Running(JoinHandle<(Client, io::Result<Status>)>),
    Closed,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    Started,
    Completed(Status),
}
pub struct Owner {
    phase: Option<Phase>,
}
impl Owner {
    pub fn start() -> io::Result<Self> {
        Self::start_with(Client::start)
    }
    pub(super) fn start_with(
        start: impl FnOnce() -> io::Result<Client> + Send + 'static,
    ) -> io::Result<Self> {
        let task = thread::Builder::new()
            .name("otter-recovery-start".into())
            .spawn(start)?;
        Ok(Self {
            phase: Some(Phase::Starting(task)),
        })
    }
    pub fn busy(&self) -> bool {
        matches!(self.phase, Some(Phase::Starting(_) | Phase::Running(_)))
    }
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        match self.phase.as_ref()? {
            Phase::Starting(task) => Some(task.as_handle()),
            Phase::Running(task) => Some(task.as_handle()),
            _ => None,
        }
    }
    /// 空闲时直接等子进程；操作期间由 worker 在管道等待中处理子进程退出。
    pub fn child_handle(&self) -> Option<BorrowedHandle<'_>> {
        match self.phase.as_ref()? {
            Phase::Idle(client) => Some(client.process_handle()),
            _ => None,
        }
    }
    pub fn healthy(&self) -> io::Result<bool> {
        match self.phase.as_ref() {
            Some(Phase::Idle(client)) => client.healthy(),
            _ => Ok(false),
        }
    }
    pub fn request(&mut self, operation: Operation) -> io::Result<()> {
        self.request_with(move |client| client.request(operation))
    }
    fn request_with(
        &mut self,
        request: impl FnOnce(&mut Client) -> io::Result<Status> + Send + 'static,
    ) -> io::Result<()> {
        if !matches!(self.phase, Some(Phase::Idle(_))) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Recovery operation already pending or unavailable",
            ));
        }
        let Some(Phase::Idle(client)) = self.phase.take() else {
            unreachable!()
        };
        // spawn 失败会丢弃闭包，因此另留一份 slot owner，确保 Client/恢复责任不随闭包丢失。
        let slot = Arc::new(Mutex::new(Some(client)));
        let input = slot.clone();
        match thread::Builder::new()
            .name("otter-recovery-request".into())
            .spawn(move || {
                let mut client = input
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take()
                    .unwrap();
                // 保留 Client 在 unwind 边界之外：未知 ACK 不得丢失原进程句柄。
                let result = catch_unwind(AssertUnwindSafe(|| request(&mut client)))
                    .unwrap_or_else(|_| {
                        client.failed = true;
                        Err(io::Error::other("Recovery request worker panicked"))
                    });
                (client, result)
            }) {
            Ok(task) => self.phase = Some(Phase::Running(task)),
            Err(error) => {
                self.phase = slot
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take()
                    .map(Phase::Idle);
                return Err(error);
            }
        }
        Ok(())
    }
    /// 仅由 Journal 在全部源恢复/取消后调用；终止请求不等于进程已退出。
    pub(super) fn stop_after_restore(&mut self) -> io::Result<()> {
        let Some(Phase::Idle(client)) = self.phase.as_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Recovery worker is not idle",
            ));
        };
        client.failed = true;
        client.child.kill()
    }
    /// 只有真实线程已退出才 join；响应至多交付一次，不在 UI 上等待 I/O。
    pub fn poll(&mut self) -> Option<io::Result<Reply>> {
        let ready = match self.phase.as_ref()? {
            Phase::Starting(task) => task.is_finished(),
            Phase::Running(task) => task.is_finished(),
            _ => false,
        };
        if !ready {
            return None;
        }
        let phase = self.phase.take()?;
        self.phase = Some(Phase::Closed);
        Some(match phase {
            Phase::Starting(task) => match task.join() {
                Ok(Ok(client)) => {
                    self.phase = Some(Phase::Idle(client));
                    Ok(Reply::Started)
                }
                Ok(Err(error)) => Err(error),
                Err(_) => Err(io::Error::other("Recovery start worker panicked")),
            },
            Phase::Running(task) => match task.join() {
                Ok((client, result)) => {
                    self.phase = Some(Phase::Idle(client));
                    result.map(Reply::Completed)
                }
                Err(_) => Err(io::Error::other("Recovery request worker panicked")),
            },
            _ => unreachable!(),
        })
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        // 最后防线不 detach。应用正常关闭应先保留 owner、泵消息并 poll 到非 busy 再 Drop。
        // 异常 Drop 可能等当前 I/O 期限/内核清理；不能声称这是 UI 非阻塞关闭。
        match self.phase.take() {
            Some(Phase::Starting(task)) => {
                let _ = task.join();
            }
            Some(Phase::Running(task)) => {
                let _ = task.join();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "live_recovery_async_tests.rs"]
mod tests;
