//! UI 线程的授权任务与凭据 owner；撤销后旧任务永远不得重新发布凭据。
use crate::{
    device_authorization_lease::{Grant, Lease},
    device_authorization_runtime::{Output, Runtime, Task},
};
use std::os::windows::io::BorrowedHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Origin,
    Pending,
    Capacity,
    Start,
    Credential,
}
pub enum Event {
    OfflineCleared {
        origin: String,
    },
    Pairing {
        origin: String,
        device: String,
        changed: bool,
    },
    Authorized {
        origin: String,
        device: String,
    },
    Failed {
        origin: String,
        error: crate::device_authorization_runtime::Error,
    },
}
struct Pending {
    origin: String,
    runtime: Runtime,
    discarded: bool,
}
#[derive(Default)]
pub struct Owner {
    pending: Vec<Pending>,
    sessions: Vec<(String, Grant)>,
    closed: bool,
    offline_denied: Vec<String>,
    offline_closed: bool,
}
impl Owner {
    /// 只对新进程尚未接触在线会话的origin允许尝试独立离线证明；容量/显式失效不能降级绕过。
    pub(crate) fn can_resume_local(&self, origin: &str) -> bool {
        !self.closed
            && !self.offline_closed
            && !self.offline_denied.iter().any(|v| v == origin)
            && !self.sessions.iter().any(|(v, _)| v == origin)
            && !self.pending.iter().any(|p| p.origin == origin)
    }
    fn deny_offline(&mut self, origin: &str) {
        if self.offline_denied.iter().any(|v| v == origin) {
            return;
        }
        if self.offline_denied.len() >= 32 {
            self.offline_closed = true;
        } else {
            self.offline_denied.push(origin.into());
        }
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn start(&mut self, task: Task) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let origin = task.origin().to_owned();
        if self
            .pending
            .iter()
            .any(|p| !p.discarded && p.origin == origin)
        {
            return Err(Error::Pending);
        }
        if self.pending.len() >= 4 {
            return Err(Error::Capacity);
        }
        // 缓存加在途最多八个不同 origin；不通过静默淘汰仍在使用的凭据来腾空间。
        let mut origins: Vec<&str> = self.sessions.iter().map(|s| s.0.as_str()).collect();
        for pending in self.pending.iter().filter(|p| !p.discarded) {
            if !origins.contains(&pending.origin.as_str()) {
                origins.push(&pending.origin);
            }
        }
        if !origins.contains(&origin.as_str()) && origins.len() >= 8 {
            return Err(Error::Capacity);
        }
        self.pending.try_reserve(1).map_err(|_| Error::Capacity)?;
        let runtime = Runtime::start(task).map_err(|_| Error::Start)?;
        self.deny_offline(&origin);
        self.pending.push(Pending {
            origin,
            runtime,
            discarded: false,
        });
        Ok(())
    }
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.pending.iter().filter_map(|p| p.runtime.wait_handle())
    }
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut index = 0;
        while index < self.pending.len() {
            let Some(result) = self.pending[index].runtime.try_join() else {
                index += 1;
                continue;
            };
            let pending = self.pending.swap_remove(index);
            if pending.discarded || self.closed {
                continue;
            }
            let origin = pending.origin;
            match result {
                Ok(Output::OfflineCleared) => events.push(Event::OfflineCleared { origin }),
                Ok(Output::Pairing { device, changed }) => {
                    // 即使 ID 未变，显式重新配对成功也必须撤销此 origin 的旧授权。
                    self.sessions.retain(|s| s.0 != origin);
                    events.push(Event::Pairing {
                        origin,
                        device,
                        changed,
                    });
                }
                Ok(Output::Session(session)) => {
                    let device = session.device_id().to_owned();
                    self.sessions.retain(|s| s.0 != origin);
                    self.sessions.push((origin.clone(), Grant::new(session)));
                    events.push(Event::Authorized { origin, device });
                }
                Err(error) => events.push(Event::Failed { origin, error }),
            }
        }
        events
    }
    pub fn token(&self, origin: &str, now_ms: u64) -> Option<&str> {
        if self.closed {
            return None;
        }
        let origin = crate::device_session_attempt::canonical_origin(origin).ok()?;
        self.sessions
            .iter()
            .find(|s| s.0 == origin)?
            .1
            .token_for(&origin, now_ms)
    }
    /// 将独立取消租约交给后台任务；不把 token 复制进 UI/消息队列。
    pub fn lease(&mut self, origin: &str, now_ms: u64) -> Result<Lease, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let origin =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        self.sessions
            .iter_mut()
            .find(|s| s.0 == origin)
            .ok_or(Error::Credential)?
            .1
            .lease(&origin, now_ms)
            .map_err(|error| match error {
                crate::device_authorization_lease::Error::Capacity => Error::Capacity,
                _ => Error::Credential,
            })
    }
    /// 服务端撤销、切换连接或关闭使用方时调用。仅影响同 origin，旧请求被永久标记丢弃。
    pub fn invalidate(&mut self, origin: &str) -> Result<(), Error> {
        let origin =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        self.deny_offline(&origin);
        self.sessions.retain(|s| s.0 != origin);
        for pending in self.pending.iter_mut().filter(|p| p.origin == origin) {
            pending.discarded = true;
            pending.runtime.request_stop();
        }
        Ok(())
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.sessions.clear();
        for pending in &mut self.pending {
            pending.discarded = true;
            pending.runtime.request_stop();
        }
    }
    pub fn finished(&self) -> bool {
        self.pending.is_empty()
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        // 正常应用退出先 close 并泵消息直到 finished；Runtime Drop 仅做阻塞兜底。
        self.close();
    }
}
