//! 有界后台 click 事务；返还唯一 owner/事件，执行证据与交付时授权分开保存。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::{Error as LeaseError, Lease},
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{ConfirmationError, ConvergenceError, SubmitError},
    surface_action_ack::Ack,
    surface_click_event::ClickEvent,
    surface_convergence::Snapshot,
    surface_lifecycle_owner::Owner,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Submit(SubmitError),
    Convergence(ConvergenceError),
    Confirmation(ConfirmationError),
    Decision(crate::surface_decision_runtime::Error),
}
pub struct Outcome {
    pub owner: Owner,
    pub event: ClickEvent,
    pub result: Result<Ack, Error>,
    /// None 表示未收到有效提交 ACK；后续失败不覆盖已经收到的 ACK。
    pub convergence: Option<Result<Snapshot, Error>>,
    /// 只在等待确认后读取一次；成功不代表当前仍可显示或提交批准。
    pub confirmation: Option<Result<crate::surface_confirmation::Request, Error>>,
    pub decision: Option<crate::surface_decision_runtime::Evidence>,
    /// ACK 是执行证据，不是当前授权。撤权后仍保留 result，但不得向 UI 应用更新。
    pub delivery: Result<(), LeaseError>,
}
pub struct StartError {
    pub owner: Box<Owner>,
    pub event: Box<ClickEvent>,
    pub error: io::Error,
}
enum Connection {
    Configured(Config),
    Prepared(Client),
}
struct Completed {
    owner: Owner,
    event: ClickEvent,
    result: Result<Ack, Error>,
    convergence: Option<Result<Snapshot, Error>>,
    confirmation: Option<Result<crate::surface_confirmation::Request, Error>>,
}
pub struct Job {
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Completed>>,
    slot: Option<Slot>,
}
impl Job {
    pub fn start(
        config: Config,
        lease: Lease,
        owner: Owner,
        revision: u64,
        event: ClickEvent,
    ) -> Result<Self, StartError> {
        Self::spawn(
            Connection::Configured(config),
            lease,
            owner,
            revision,
            event,
        )
    }
    pub fn start_prepared(
        client: Client,
        lease: Lease,
        owner: Owner,
        revision: u64,
        event: ClickEvent,
    ) -> Result<Self, StartError> {
        Self::spawn(Connection::Prepared(client), lease, owner, revision, event)
    }
    fn spawn(
        connection: Connection,
        lease: Lease,
        owner: Owner,
        revision: u64,
        event: ClickEvent,
    ) -> Result<Self, StartError> {
        let slot = match Slot::acquire() {
            Ok(slot) => slot,
            Err(error) => {
                return Err(StartError {
                    owner: Box::new(owner),
                    event: Box::new(event),
                    error,
                })
            }
        };
        // spawn 失败也要完整返还输入；worker 开始后独占 owner 和事件，绝不复制提交。
        let transfer = Arc::new(Mutex::new(Some((owner, event))));
        let worker = transfer.clone();
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-event".into())
            .spawn(move || {
                let (owner, mut event) = worker
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("single event transfer");
                let mut convergence = None;
                let mut confirmation = None;
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
                    let ack = control
                        .submit_surface_click(&owner, revision, &mut event)
                        .map_err(Error::Submit)?;
                    // 独立捕获收敛阶段异常，避免丢失已收到的提交证据。
                    let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        control.converge_surface(
                            &event,
                            &ack,
                            Instant::now() + Duration::from_secs(25),
                        )
                    }));
                    match observed {
                        Ok(outcome) => {
                            let mut ack = outcome.ack;
                            if matches!(outcome.result, Err(ConvergenceError::ConfirmationRequired))
                            {
                                // 后续读取异常也不能抹掉已确认的等待状态，不重放原动作。
                                let prompt =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                        control.surface_confirmation(&event, &ack)
                                    }));
                                match prompt {
                                    Ok(prompt) => {
                                        confirmation =
                                            Some(prompt.result.map_err(Error::Confirmation));
                                        ack = prompt.ack;
                                    }
                                    Err(_) => confirmation = Some(Err(Error::Panicked)),
                                }
                            }
                            convergence = Some(outcome.result.map_err(Error::Convergence));
                            Ok(ack)
                        }
                        Err(_) => {
                            convergence = Some(Err(Error::Panicked));
                            Ok(ack)
                        }
                    }
                }))
                .unwrap_or(Err(Error::Panicked));
                Completed {
                    owner,
                    event,
                    result,
                    convergence,
                    confirmation,
                }
            });
        match thread {
            Ok(thread) => Ok(Self {
                lease,
                thread: Some(thread),
                slot: Some(slot),
            }),
            Err(error) => {
                let (owner, event) = transfer
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("unstarted event");
                Err(StartError {
                    owner: Box::new(owner),
                    event: Box::new(event),
                    error,
                })
            }
        }
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    /// 调用方传入当前时刻，完成线程仍占共享容量，直到被显式收割。
    pub fn try_join(&mut self, now_ms: u64) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let completed = self
            .thread
            .take()?
            .join()
            .expect("worker contains submission panic");
        self.slot.take();
        let delivery = self
            .lease
            .validate(completed.owner.binding().origin(), now_ms);
        Some(Outcome {
            owner: completed.owner,
            event: completed.event,
            result: completed.result,
            convergence: completed.convergence,
            confirmation: completed.confirmation,
            decision: None,
            delivery,
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // 正常应用路径必须先 wait/try_join 保存证据；兜底确保没有遗留线程。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
