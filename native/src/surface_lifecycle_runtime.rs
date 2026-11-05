//! Surface 后台事务；完成回收必返还 owner，失败/取消不得丢失 unknown 后伪造新的已知状态。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{Error as HttpError, MountedSurface},
    surface_lifecycle_owner::Owner,
    surface_live_binding::Lifecycle,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
pub enum Action {
    Transition(Lifecycle),
    Recover,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Http(HttpError),
}
pub struct Outcome {
    pub owner: Owner,
    pub result: Result<Option<MountedSurface>, Error>,
}
pub struct StartError {
    pub owner: Box<Owner>,
    pub error: io::Error,
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
    pub fn start(
        config: Config,
        lease: Lease,
        owner: Owner,
        action: Action,
    ) -> Result<Self, StartError> {
        Self::spawn(Connection::Configured(config), lease, owner, action)
    }
    pub fn start_prepared(
        client: Client,
        lease: Lease,
        owner: Owner,
        action: Action,
    ) -> Result<Self, StartError> {
        Self::spawn(Connection::Prepared(client), lease, owner, action)
    }
    fn spawn(
        connection: Connection,
        lease: Lease,
        owner: Owner,
        action: Action,
    ) -> Result<Self, StartError> {
        let slot = match Slot::acquire() {
            Ok(slot) => slot,
            Err(error) => {
                return Err(StartError {
                    owner: Box::new(owner),
                    error,
                })
            }
        };
        // spawn 失败时闭包不会运行，保留另一引用取回 owner；线程开始后只由 worker 持有。
        let transfer = Arc::new(Mutex::new(Some(owner)));
        let worker = transfer.clone();
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-lifecycle".into())
            .spawn(move || {
                let mut owner = worker
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("single owner transfer");
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if task.is_revoked() {
                        return Err(Error::Stopped);
                    }
                    let client = match connection {
                        Connection::Prepared(client) => client,
                        Connection::Configured(config) => {
                            task.prepare_client(&config).map_err(Error::Preparation)?
                        }
                    };
                    let control = task.control(&client).map_err(|_| Error::Stopped)?;
                    match action {
                        Action::Transition(state) => {
                            control.transition_surface(&mut owner, state).map(|()| None)
                        }
                        Action::Recover => control.recover_surface(&mut owner).map(Some),
                    }
                    .map_err(Error::Http)
                }));
                let result = match result {
                    Ok(result) => result,
                    Err(_) => {
                        let _ = owner.begin_recovery();
                        Err(Error::Panicked)
                    }
                };
                Outcome { owner, result }
            });
        match thread {
            Ok(thread) => Ok(Self {
                lease,
                thread: Some(thread),
                slot: Some(slot),
            }),
            Err(error) => Err(StartError {
                owner: Box::new(
                    transfer
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .take()
                        .expect("unstarted owner"),
                ),
                error,
            }),
        }
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
            .expect("worker contains operation panic");
        self.slot.take();
        // 撤权后不给 UI 交付可用的场景，但仍返还已知/unknown owner 作为恢复证据。
        if self.lease.is_revoked() {
            outcome.result = Err(Error::Stopped);
        }
        Some(outcome)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // 正常 owner 必须 wait/try_join 保存结果；兜底禁止遗留后台线程。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
