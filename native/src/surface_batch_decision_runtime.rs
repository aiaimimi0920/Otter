//! 批次内单项决定 worker：完整批次证据随 owner 返回，不自动提交剩余点击。
pub use crate::{
    desktop::surface_authorized_window::BatchDecisionInput as Input,
    surface_decision_runtime::Error,
};
use crate::{
    device_authorization_config::Config,
    device_authorization_lease::{Error as LeaseError, Lease},
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::DecisionSubmitError,
    surface_convergence::Snapshot,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub struct Outcome {
    pub input: Input,
    pub submission: Result<(), Error>,
    pub convergence: Option<Result<Snapshot, Error>>,
    pub delivery: Result<(), LeaseError>,
}
#[path = "surface_batch_decision_merge.rs"]
mod merge;
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
            .name("otter-surface-batch-decision".into())
            .spawn(move || {
                let mut input = worker
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("single batch decision transfer");
                let mut convergence = None;
                let submission = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if task.is_revoked() {
                        return Err(Error::Stopped);
                    }
                    let prepared = &mut input.prepared;
                    let receipt = &mut prepared.receipt;
                    if receipt.revision != input.revision
                        || crate::live_relay_http::validate_batch_identity(
                            &input.owner,
                            input.revision,
                            &receipt.events,
                        )
                        .is_err()
                    {
                        return Err(Error::Submit(DecisionSubmitError::Identity));
                    }
                    let event = receipt
                        .events
                        .get(prepared.index)
                        .ok_or(Error::Submit(DecisionSubmitError::Identity))?;
                    let ack = receipt
                        .submission
                        .as_mut()
                        .ok()
                        .and_then(|value| value.acknowledgements.get_mut(prepared.index))
                        .ok_or(Error::Submit(DecisionSubmitError::Identity))?;
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
                            event,
                            ack,
                            &mut prepared.decision,
                        )
                        .map_err(Error::Submit)?;
                    // 已接收决定与动作收敛分开；后续 panic 不抹去 HTTP 接收事实。
                    convergence = Some(
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            control.converge_decided_surface(
                                event,
                                ack,
                                &prepared.decision,
                                Instant::now() + Duration::from_secs(25),
                            )
                        })) {
                            Ok(outcome) => {
                                *ack = outcome.ack;
                                outcome.result.map_err(Error::Convergence)
                            }
                            Err(_) => Err(Error::Panicked),
                        },
                    );
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
            Err(error) => Err(StartError {
                input: Box::new(
                    transfer
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .take()
                        .expect("unstarted batch decision"),
                ),
                error,
            }),
        }
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn wait_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.thread.as_ref().map(|thread| thread.as_handle())
    }
    pub fn try_join(&mut self, now_ms: u64) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let completed = self
            .thread
            .take()?
            .join()
            .expect("worker contains batch decision panic");
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
        // 正常路径显式收割证据；兜底不遗留线程，不代表远端操作被撤回。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
