//! 创建或显式候选恢复的后台挂载事务；取消不代表远端回滚，不自动重发 POST。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{Error as HttpError, Persistence, RecoveryCandidate},
    surface_host_capabilities::Report,
    surface_live_binding::Binding,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::Arc,
    thread::JoinHandle,
};
pub struct Request {
    art: String,
    node: String,
    report: Report,
    operation: Operation,
}
enum Operation {
    Create(Persistence),
    Recover(Arc<RecoveryCandidate>),
}
impl Request {
    pub fn new(
        art: String,
        node: String,
        report: Report,
        persistence: Persistence,
    ) -> io::Result<Self> {
        if !crate::live_relay_viewer_identity::identifier(&art)
            || !crate::live_relay_viewer_identity::identifier(&node)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            art,
            node,
            report,
            operation: Operation::Create(persistence),
        })
    }
    /// 只接受认证查询产生的候选；调用方显式选择恢复，不隐式重建原 Art。
    pub fn recover(candidate: Arc<RecoveryCandidate>, report: Report) -> Self {
        Self {
            art: candidate.art().to_owned(),
            node: candidate.node().to_owned(),
            report,
            operation: Operation::Recover(candidate),
        }
    }
    pub fn recovery_candidate(&self) -> Option<&RecoveryCandidate> {
        match &self.operation {
            Operation::Recover(candidate) => Some(candidate),
            Operation::Create(_) => None,
        }
    }
    pub fn art(&self) -> &str {
        &self.art
    }
    pub fn node(&self) -> &str {
        &self.node
    }
    pub fn supports(&self, runtime: crate::surface_host_capabilities::Runtime) -> bool {
        self.report.supports(runtime)
    }
}
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
    /// 有效响应已经确认的绑定，即使迟到撤权也保留，不代表当前仍有操作授权。
    pub binding: Option<Binding>,
    pub result: Result<Box<serde_json::value::RawValue>, Error>,
    /// 已尝试挂载但没有确认绑定；必须保留上下文，禁止自动重试创建或恢复。
    pub uncertain: bool,
}
#[path = "surface_mount_admission.rs"]
mod admission;
pub use admission::{AdmissionError, AdmittedSnapshot};
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
            .name("otter-surface-attach".into())
            .spawn(move || {
                let mut attempted = false;
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
                    let control = task.control(&client).map_err(|_| Error::Stopped)?;
                    attempted = true;
                    match &request.operation {
                        Operation::Create(persistence) => control.attach_surface(
                            &request.art,
                            &request.node,
                            &request.report,
                            *persistence,
                        ),
                        Operation::Recover(candidate) => {
                            control.remount_surface_candidate(candidate, &request.report)
                        }
                    }
                    .map_err(Error::Http)
                }))
                .unwrap_or(Err(Error::Panicked));
                let (binding, result) = match result {
                    Ok(mounted) => (Some(mounted.binding), Ok(mounted.snapshot)),
                    Err(error) => (None, Err(error)),
                };
                Outcome {
                    origin,
                    device,
                    request,
                    uncertain: attempted && binding.is_none(),
                    binding,
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
        let mut outcome = self
            .thread
            .take()?
            .join()
            .expect("worker contains attach panic");
        self.slot.take();
        if self.lease.is_revoked() {
            outcome.result = Err(Error::Stopped);
        }
        Some(outcome)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // 上层正常退出必须先 wait/try_join 保存恢复证据；Drop 不承诺撤销远端创建。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
