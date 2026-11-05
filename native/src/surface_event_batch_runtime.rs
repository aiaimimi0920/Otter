//! 有界批次提交 worker；整批原事件和唯一 owner 随结果返还，提交不是执行完成。
use crate::{
    device_authorization_config::{Config, Error as PrepareError},
    device_authorization_lease::{Error as LeaseError, Lease},
    device_session_client::Client,
    live_relay_control_runtime::Slot,
    live_relay_http::{BatchError, BatchSubmission},
    surface_click_event::ClickEvent,
    surface_lifecycle_owner::Owner,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
#[path = "surface_event_batch_observation.rs"]
mod observation;
pub use observation::{Error as ObservationError, Item as ObservedItem, Observation};
#[path = "surface_batch_continuation_runtime.rs"]
pub mod continuation;
pub struct Input {
    pub owner: Owner,
    pub revision: u64,
    pub events: Vec<ClickEvent>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotStarted,
    Allocation,
    Stopped,
    Panicked,
    Preparation(PrepareError),
    Batch(BatchError),
}
pub struct Outcome {
    pub input: Input,
    pub submission: Result<BatchSubmission, Error>,
    pub delivery: Result<(), LeaseError>,
    pub observation: Option<Result<Observation, Error>>,
}
pub struct StartError {
    pub input: Box<Input>,
    pub error: io::Error,
}
struct Completed {
    input: Input,
    submission: Result<BatchSubmission, Error>,
    observation: Option<Result<Observation, Error>>,
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
            .name("otter-surface-click-batch".into())
            .spawn(move || {
                let mut input = worker
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("single batch transfer");
                let mut observation = None;
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
                    let mut submitted = control
                        .submit_surface_click_batch(&input.owner, input.revision, &mut input.events)
                        .map_err(Error::Batch)?;
                    // 后续观察失败不能改写提交事实，也不重新发送任何事件。
                    observation = Some(
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            observation::observe(&control, &input.events, &mut submitted)
                        }))
                        .unwrap_or(Err(Error::Panicked)),
                    );
                    Ok(submitted)
                }))
                .unwrap_or(Err(Error::Panicked));
                Completed {
                    input,
                    submission,
                    observation,
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
                    .expect("unstarted batch");
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
    /// 完成仍占共享槽位，显式收割才释放；迟到撤权不能抹去已提交证据。
    pub fn try_join(&mut self, now_ms: u64) -> Option<Outcome> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let completed = self
            .thread
            .take()?
            .join()
            .expect("worker contains batch panic");
        self.slot.take();
        let delivery = self
            .lease
            .validate(completed.input.owner.binding().origin(), now_ms);
        Some(Outcome {
            input: completed.input,
            submission: completed.submission,
            delivery,
            observation: completed.observation,
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        // 正常路径必须先收割保存证据；析构兜底不遗留线程。
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
