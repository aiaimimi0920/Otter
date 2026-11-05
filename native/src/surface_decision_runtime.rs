//! 决定提交后的有界后台事务；唯一 owner 与决定证据完整返回，不重放原事件。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::{Error as LeaseError, Lease},
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{ConvergenceError, DecisionSubmitError},
    surface_action_ack::Ack,
    surface_click_event::ClickEvent,
    surface_confirmation::Decision,
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
pub struct Input {
    pub owner: Owner,
    pub event: ClickEvent,
    pub ack: Ack,
    pub decision: Decision,
    /// 从当前呈现 owner 读取的版本，不是重新取 event.revision 冒充检查。
    pub revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// 未创建后台任务；原输入仍通过统一结果链路返还，不代表网络尝试失败。
    NotStarted,
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Submit(DecisionSubmitError),
    Convergence(ConvergenceError),
}
pub struct Outcome {
    pub input: Input,
    /// HTTP 决定提交结果，不代表动作执行结果。
    pub submission: Result<(), Error>,
    pub convergence: Option<Result<Snapshot, Error>>,
    pub delivery: Result<(), LeaseError>,
}
/// 进入统一事件收据的决定证据；与动作 ACK/呈现结果分开，不能被错误提示消费。
pub struct Evidence {
    pub decision: Decision,
    pub revision: u64,
    pub submission: Result<(), Error>,
}
impl Outcome {
    pub fn into_event(self) -> crate::surface_event_runtime::Outcome {
        let Input {
            owner,
            event,
            ack,
            decision,
            revision,
        } = self.input;
        crate::surface_event_runtime::Outcome {
            owner,
            event,
            result: Ok(ack),
            convergence: self
                .convergence
                .map(|value| value.map_err(crate::surface_event_runtime::Error::Decision)),
            confirmation: None,
            delivery: self.delivery,
            decision: Some(Evidence {
                decision,
                revision,
                submission: self.submission,
            }),
        }
    }
}
pub struct StartError {
    pub input: Box<Input>,
    pub error: io::Error,
}
struct Completed {
    input: Input,
    submission: Result<(), Error>,
    convergence: Option<Result<Snapshot, Error>>,
}
enum Connection {
    Configured(Config),
    Prepared(Client),
}
pub struct Job {
    lease: Arc<Lease>,
    thread: Option<JoinHandle<Completed>>,
    slot: Option<Slot>,
}
impl Job {
    pub fn start(config: Config, lease: Lease, input: Input) -> Result<Self, StartError> {
        Self::spawn(Connection::Configured(config), lease, input)
    }
    pub fn start_prepared(client: Client, lease: Lease, input: Input) -> Result<Self, StartError> {
        Self::spawn(Connection::Prepared(client), lease, input)
    }
    fn spawn(connection: Connection, lease: Lease, input: Input) -> Result<Self, StartError> {
        let slot = match Slot::acquire() {
            Ok(slot) => slot,
            Err(error) => {
                return Err(StartError {
                    input: Box::new(input),
                    error,
                })
            }
        };
        let transfer = Arc::new(Mutex::new(Some(input)));
        let worker = transfer.clone();
        let lease = Arc::new(lease);
        let task = lease.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-decision".into())
            .spawn(move || {
                let mut input = worker
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("single decision transfer");
                let mut convergence = None;
                let submission = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                    control
                        .submit_surface_confirmation(
                            &input.owner,
                            input.revision,
                            &input.event,
                            &input.ack,
                            &mut input.decision,
                        )
                        .map_err(Error::Submit)?;
                    // 提交成功和后续收敛是两份证据；后续异常不能改写决定接收事实。
                    let observed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        control.converge_decided_surface(
                            &input.event,
                            &input.ack,
                            &input.decision,
                            Instant::now() + Duration::from_secs(25),
                        )
                    }));
                    convergence = Some(match observed {
                        Ok(outcome) => {
                            input.ack = outcome.ack;
                            outcome.result.map_err(Error::Convergence)
                        }
                        Err(_) => Err(Error::Panicked),
                    });
                    Ok(())
                }))
                .unwrap_or(Err(Error::Panicked));
                Completed {
                    input,
                    submission,
                    convergence,
                }
            });
        match thread {
            Ok(thread) => Ok(Self {
                lease,
                thread: Some(thread),
                slot: Some(slot),
            }),
            Err(error) => {
                let input = transfer
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("unstarted decision");
                Err(StartError {
                    input: Box::new(input),
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
    /// 完成线程仍占共享槽位，直到 UI 显式收割；结果使用前仍须再次核验授权。
    pub fn try_join(&mut self, now_ms: u64) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let completed = self
            .thread
            .take()?
            .join()
            .expect("worker contains decision panic");
        self.slot.take();
        let delivery = self
            .lease
            .validate(completed.input.owner.binding().origin(), now_ms);
        Some(Outcome {
            input: completed.input,
            submission: completed.submission,
            convergence: completed.convergence,
            delivery,
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // 正常应用路径先 wait/try_join 保存证据；兜底不遗留后台线程。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
