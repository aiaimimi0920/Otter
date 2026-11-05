//! 单次传统投送后台任务；复用原四任务容量与租约，完成未领取仍占槽，不在 UI 做 CA/DNS。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::Error as HttpError,
    projection_delivery::{Inbox, InboxPolicy, Invitation},
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::Arc,
    thread::JoinHandle,
};
#[path = "projection_receive_ready.rs"]
mod received;
pub use received::Ready;
#[path = "projection_receive_hold.rs"]
mod hold;
pub use hold::Hold;
#[path = "projection_delivery_work.rs"]
mod work;
use work::Work;
#[path = "projection_receive_attempt.rs"]
mod attempt;
#[path = "projection_receive_recovery.rs"]
mod recovery;
#[path = "projection_sender_ready.rs"]
mod sent;
#[path = "projection_receive_sync.rs"]
pub(crate) mod sync;
pub use sent::Ready as SenderReady;
#[path = "projection_sender_flow.rs"]
mod sender_flow;
pub(crate) use sender_flow::{SenderInput, SenderPreparation};
#[path = "projection_edit_submit.rs"]
pub(crate) mod edit;
#[path = "projection_sender_update_flow.rs"]
pub(crate) mod sender_update;

pub enum Operation {
    Targets,
    Inbox(InboxPolicy),
    Reject(Box<Invitation>),
    // profile owner 在后台打开一次；Ticket 与并行任务共享同一个独占 Store。
    Accept {
        invitation: Box<Invitation>,
        journal: crate::projection_delivery::journal::Store,
    },
    // 仅恢复服务端既有 receiver；不发送 accept，不获得自动补偿资格。
    Reattach {
        invitation: Box<Invitation>,
        journal: crate::projection_delivery::journal::Store,
    },
    Compensate(Box<crate::projection_delivery::journal::Ticket>),
    Send(Box<crate::projection_delivery::sender::Request>),
    CancelSend(Box<crate::projection_delivery::sender::Request>),
}
#[derive(Debug)]
pub enum Output {
    Targets(crate::projection_delivery::targets::Directory),
    Inbox(Inbox),
    Rejected,
    Received(Box<Ready>),
    Compensated,
    Displayed,
    Synchronized(Box<sync::Outcome>),
    Sent(Box<SenderReady>),
    SenderUpdated(Box<SenderReady>),
    SenderUnlinked,
    Edited(Box<edit::Ready>),
    EditAcknowledged,
}
enum Completed {
    Metadata(Output),
    Accepted(Box<crate::projection_delivery::accept::Verified>),
    Sent(Box<sent::Connected>),
    Updated(Box<sent::Connected>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Http(HttpError),
    Decode(crate::projection_delivery::Error),
    Journal(crate::projection_delivery::journal::Error),
    Sender(crate::projection_delivery::sender::Error),
    Edit(crate::projection_edit::Failure),
}
enum Connection {
    Configured(Config),
    Prepared(Client),
}
pub struct Job {
    origin: String,
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Result<Completed, Error>>>,
    slot: Option<Slot>,
    transferred: bool,
    attempt: Arc<attempt::Attempt>,
    sender_attempt: Arc<sender_flow::Attempt>,
    // displayed 接续原 Hold，不重取第五个 Slot 或再次 activate 同 intent。
    _held: Option<Hold>,
}
impl Job {
    pub fn start(config: Config, lease: Lease, operation: Operation) -> io::Result<Self> {
        Self::spawn(Connection::Configured(config), lease, operation)
    }
    pub fn start_prepared(client: Client, lease: Lease, operation: Operation) -> io::Result<Self> {
        Self::spawn(Connection::Prepared(client), lease, operation)
    }
    fn spawn(connection: Connection, lease: Lease, operation: Operation) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        Self::spawn_admitted(
            connection,
            Arc::new(lease),
            Work::Request(operation),
            Some(slot),
            None,
        )
    }
    pub(crate) fn start_displayed(config: Config, held: Hold) -> io::Result<Self> {
        if config.origin() != held.origin() || now_ms().is_none_or(|now| held.check(now).is_err()) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let work = Work::Displayed(Box::new(held.ticket().retry()));
        Self::spawn_admitted(
            Connection::Configured(config),
            held.lease.clone(),
            work,
            None,
            Some(held),
        )
    }
    pub(crate) fn start_recovery(
        config: Config,
        lease: Lease,
        ticket: crate::projection_delivery::journal::Ticket,
        expected: crate::projection_delivery::binding::Binding,
        image: crate::image_decode::Image,
    ) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::Recover(Box::new(recovery::Request {
                ticket,
                expected,
                image,
            })),
            Some(Slot::acquire()?),
            None,
        )
    }
    pub(crate) fn expires_at_ms(&self) -> Option<u64> {
        (!self.lease.is_revoked()).then(|| self.lease.expires_at_ms())
    }
    pub(crate) fn start_sync(
        config: Config,
        lease: Lease,
        request: sync::Request,
    ) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::Sync(Box::new(request)),
            Some(Slot::acquire()?),
            None,
        )
    }
    fn spawn_admitted(
        connection: Connection,
        lease: Arc<Lease>,
        work: Work,
        slot: Option<Slot>,
        held: Option<Hold>,
    ) -> io::Result<Self> {
        let origin = match &connection {
            Connection::Configured(config) => config.origin(),
            Connection::Prepared(client) => client.origin(),
        }
        .to_owned();
        let task = lease.clone();
        let attempt = Arc::new(attempt::Attempt::default());
        let worker_attempt = attempt.clone();
        let sender_attempt = Arc::new(sender_flow::Attempt::default());
        if let Work::Request(Operation::Send(request) | Operation::CancelSend(request)) = &work {
            sender_attempt.record((**request).clone());
        }
        let worker_sender = sender_attempt.clone();
        let thread = std::thread::Builder::new()
            .name("otter-projection-inbox".into())
            .spawn(move || {
                if task.is_revoked() {
                    return Err(Error::Stopped);
                }
                // 精确Checkpoint后只写本地日志，不重新发现CA/DNS或发第二次HTTP。
                if let Work::EditAck(ready) = work {
                    ready.ack(&task)?;
                    return Ok(Completed::Metadata(Output::EditAcknowledged));
                }
                let client = match connection {
                    Connection::Prepared(client) => client,
                    Connection::Configured(config) => {
                        task.prepare_client(&config).map_err(Error::Preparation)?
                    }
                };
                task.validate(client.origin(), now_ms().ok_or(Error::Stopped)?)
                    .map_err(|_| Error::Stopped)?;
                let control = task.control(&client).map_err(|_| Error::Stopped)?;
                let output = work.run(&control, &client, &task, &worker_attempt, &worker_sender)?;
                // 完整 outbox 基线只在原 worker 核验；UI join 只领取内存结果和当前租约。
                if let Completed::Sent(value) | Completed::Updated(value) = &output {
                    value.active.check().map_err(Error::Sender)?;
                }
                if task.is_revoked() {
                    return Err(Error::Stopped);
                }
                Ok(output)
            })?;
        Ok(Self {
            origin,
            lease,
            thread: Some(thread),
            slot,
            transferred: false,
            attempt,
            sender_attempt,
            _held: held,
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub(crate) fn held(&self) -> Option<&Hold> {
        self._held.as_ref()
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Result<Output, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        self.join()
    }
    /// 只供最后 Unit/应用终态；普通取消必须继续等待原句柄，不在 UI join。
    pub(crate) fn stop_and_join(&mut self) {
        self.request_stop();
        let _ = self.join();
    }
    pub(crate) fn take_abandoned(&mut self) -> Option<crate::projection_delivery::journal::Ticket> {
        if self.thread.is_some() || self.transferred {
            return None;
        }
        self.attempt.abandon()
    }
    fn join(&mut self) -> Option<Result<Output, Error>> {
        let result = self.thread.take()?.join().unwrap_or(Err(Error::Panicked));
        let now = now_ms();
        // 完成后的撤销或到期也不能发布；调用方仍须另核工作区/选择的生命周期。
        let result = if now.is_none_or(|now| self.lease.validate(&self.origin, now).is_err()) {
            Err(Error::Stopped)
        } else {
            result.and_then(|completed| match completed {
                Completed::Metadata(output) => {
                    // Source Ready继续持有原Active/Lease直到Checkpoint和ack；已join的Job
                    // 不得在Drop撤销移交的租约。其他metadata没有此owner，保持原取消回收。
                    self.transferred =
                        matches!(&output, Output::Edited(ready) if ready.source.is_some());
                    Ok(output)
                }
                Completed::Accepted(value) => {
                    let (intent, fresh) = self.attempt.take().ok_or(Error::Journal(
                        crate::projection_delivery::journal::Error::Changed,
                    ))?;
                    self.transferred = true;
                    Ok(Output::Received(Box::new(Ready::new(
                        *value,
                        self.lease.clone(),
                        self.slot.take().unwrap(),
                        intent,
                        fresh,
                    ))))
                }
                Completed::Sent(value) => {
                    self.transferred = true;
                    Ok(Output::Sent(Box::new(SenderReady::new(
                        *value,
                        self.slot.take().unwrap(),
                    ))))
                }
                Completed::Updated(value) => {
                    self.transferred = true;
                    Ok(Output::SenderUpdated(Box::new(SenderReady::new(
                        *value,
                        self.slot.take().unwrap(),
                    ))))
                }
            })
        };
        self.slot.take();
        Some(result)
    }
}
fn now_ms() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|time| u64::try_from(time.as_millis()).ok())
}
impl Drop for Job {
    fn drop(&mut self) {
        if !self.transferred {
            self.request_stop();
        }
        // 正常 owner 使用 wait/try_join；兜底不 detach，取消会中断原串行网络链。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
#[path = "projection_receive_attempt_tests.rs"]
mod attempt_tests;
