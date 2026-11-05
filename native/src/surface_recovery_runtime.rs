//! 只读恢复查询的后台生命周期；结果不修改原挂载 unknown，不自动接受候选。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{Error as HttpError, RecoveryCandidate},
    surface_attach_runtime::Request,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::Arc,
    thread::JoinHandle,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Http(HttpError),
}
pub struct Outcome {
    pub origin: String,
    pub device: String,
    pub request: Arc<Request>,
    /// None 只是未找到候选，不能作为原创建没有生效的证据。
    pub result: Result<Option<RecoveryCandidate>, Error>,
}
enum Connection {
    Configured(Config),
    Prepared(Client),
}
pub struct Job {
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Outcome>>,
    slot: Option<Slot>,
}
impl Job {
    pub fn start(config: Config, lease: Lease, request: Arc<Request>) -> io::Result<Self> {
        Self::spawn(Connection::Configured(config), lease, request)
    }
    pub fn start_prepared(client: Client, lease: Lease, request: Arc<Request>) -> io::Result<Self> {
        Self::spawn(Connection::Prepared(client), lease, request)
    }
    fn spawn(connection: Connection, lease: Lease, request: Arc<Request>) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let origin = match &connection {
            Connection::Configured(c) => c.origin(),
            Connection::Prepared(c) => c.origin(),
        }
        .to_owned();
        let device = lease.device_id().to_owned();
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-recovery".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if task.is_revoked() {
                        return Err(Error::Stopped);
                    }
                    let client = match connection {
                        Connection::Prepared(c) => c,
                        Connection::Configured(c) => {
                            task.prepare_client(&c).map_err(Error::Preparation)?
                        }
                    };
                    task.control(&client)
                        .map_err(|_| Error::Stopped)?
                        .surface_recovery_candidate(request.art(), request.node())
                        .map_err(Error::Http)
                }))
                .unwrap_or(Err(Error::Panicked));
                Outcome {
                    origin,
                    device,
                    request,
                    result,
                }
            })?;
        Ok(Self {
            lease,
            thread: Some(thread),
            slot: Some(slot),
        })
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|t| t.as_handle())
    }
    pub fn try_join(&mut self) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let mut result = self
            .thread
            .take()?
            .join()
            .expect("worker contains recovery panic");
        self.slot.take();
        if self.lease.is_revoked() {
            result.result = Err(Error::Stopped);
        }
        Some(result)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // 正常应用流程等待完成再回收，Drop 仅作不遗留线程的兜底。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
