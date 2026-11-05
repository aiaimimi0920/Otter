//! 恢复子进程的有界登记状态机；传输须已认证，bind 必须绑定原源进程身份并保活句柄。
//! 使用单请求在途协议：未收到 ACK 时只能原样重发，不能流水线发送下一序号。
use crate::live_recovery_wire::{Ack, Command, Record, Request, Status, MAX_RECORDS, SCHEMA};
use std::io;

struct Entry<T> {
    id: u64,
    record: Record,
    owner: T,
}
pub struct Service<T> {
    session: [u8; 16],
    entries: Vec<Entry<T>>,
    last: Option<(Request, Ack)>,
    highest_id: u64,
    shutdown: bool,
    recovering: bool,
}
fn protocol(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl<T> Service<T> {
    pub fn new(session: [u8; 16]) -> io::Result<Self> {
        if session == [0; 16] {
            return Err(protocol("Invalid Live recovery session"));
        }
        Ok(Self {
            session,
            entries: Vec::new(),
            last: None,
            highest_id: 0,
            shutdown: false,
            recovering: false,
        })
    }
    pub fn pending(&self) -> usize {
        self.entries.len()
    }
    pub fn shutdown_accepted(&self) -> bool {
        self.shutdown
    }
    pub fn apply(
        &mut self,
        request: Request,
        bind: impl FnOnce(&Record) -> io::Result<T>,
    ) -> io::Result<Ack> {
        request.validate()?;
        if request.session != self.session || self.recovering {
            return Err(protocol("Inactive Live recovery session"));
        }
        if let Some((previous, ack)) = &self.last {
            if request.sequence == previous.sequence {
                return if &request == previous {
                    Ok(ack.clone())
                } else {
                    Err(protocol("Changed Live recovery retry"))
                };
            }
        }
        let expected = self
            .last
            .as_ref()
            .map_or(Some(1), |(request, _)| request.sequence.checked_add(1));
        if self.shutdown || Some(request.sequence) != expected {
            return Err(protocol("Out-of-order Live recovery request"));
        }
        let accepted = match &request.command {
            Command::Register { id, record } => {
                if *id <= self.highest_id {
                    false
                } else {
                    // 即使本次拒绝也消费 id；退休后不可用旧 id 重新登记另一代窗口状态。
                    self.highest_id = *id;
                    if self.entries.len() >= MAX_RECORDS
                        || self
                            .entries
                            .iter()
                            .any(|entry| entry.record.hwnd == record.hwnd)
                    {
                        false
                    } else {
                        self.entries.try_reserve(1).map_err(io::Error::other)?;
                        match bind(record) {
                            Ok(owner) => {
                                self.entries.push(Entry {
                                    id: *id,
                                    record: record.clone(),
                                    owner,
                                });
                                true
                            }
                            Err(_) => false,
                        }
                    }
                }
            }
            Command::Retire { id } => {
                if *id > self.highest_id {
                    false
                } else {
                    self.entries.retain(|entry| entry.id != *id);
                    true
                }
            }
            Command::Shutdown => {
                self.shutdown = self.entries.is_empty();
                self.shutdown
            }
        };
        let ack = Ack {
            schema: SCHEMA,
            session: self.session,
            sequence: request.sequence,
            status: if accepted {
                Status::Accepted
            } else {
                Status::Rejected
            },
        };
        self.last = Some((request, ack.clone()));
        Ok(ack)
    }
    /// 仅在可信 owner 确认应进入终止恢复后调用；不能仅因 EOF 猜测父进程已死。
    /// 成功记录释放，失败记录和原源 owner 保留供重试；进入恢复后不接受任何新命令。
    pub fn recover(
        &mut self,
        mut restore: impl FnMut(&Record, &mut T) -> io::Result<()>,
    ) -> io::Result<()> {
        self.recovering = true;
        let mut first = None;
        let mut index = 0;
        while index < self.entries.len() {
            let entry = &mut self.entries[index];
            match restore(&entry.record, &mut entry.owner) {
                Ok(()) => {
                    self.entries.remove(index);
                }
                Err(error) => {
                    first.get_or_insert(error);
                    index += 1;
                }
            }
        }
        first.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
#[path = "live_recovery_service_tests.rs"]
mod tests;
