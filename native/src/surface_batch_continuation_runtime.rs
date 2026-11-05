//! 显式接续 worker：完整收据随唯一 owner 返回，只提交未经尝试的后缀。
use super::*;
pub use crate::desktop::surface_authorized_window::BatchContinuationInput as Input;
pub struct Outcome {
    pub input: Input,
    pub result: Result<(), Error>,
    pub delivery: Result<(), LeaseError>,
}
pub struct StartError {
    pub input: Box<Input>,
    pub error: io::Error,
}
struct Completed {
    input: Input,
    result: Result<(), Error>,
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
            .name("otter-surface-batch-continuation".into())
            .spawn(move || {
                let mut input = worker
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("single continuation transfer");
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
                    execute(&control, &mut input)
                }))
                .unwrap_or(Err(Error::Panicked));
                Completed { input, result }
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
                        .expect("unstarted continuation"),
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
            .expect("contained continuation panic");
        self.slot.take();
        let delivery = self
            .lease
            .validate(completed.input.owner.binding().origin(), now_ms);
        Some(Outcome {
            input: completed.input,
            result: completed.result,
            delivery,
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn execute(control: &crate::live_relay_http::Control<'_>, input: &mut Input) -> Result<(), Error> {
    let receipt = &mut input.receipt;
    if input.revision != receipt.revision || receipt.continuation_boundary() != Some(input.next) {
        return Err(Error::Batch(BatchError::Identity));
    }
    crate::live_relay_http::validate_batch_identity(&input.owner, input.revision, &receipt.events)
        .map_err(Error::Batch)?;
    let submitted = receipt.submission.as_mut().expect("validated submission");
    let observed = receipt
        .observation
        .as_mut()
        .expect("validated observation")
        .as_mut()
        .expect("validated result");
    let remaining = receipt.events.len() - input.next;
    // 网络副作用前为原批次追加结果预留空间，不能在接收 ACK 后才发现无法保存。
    submitted
        .acknowledgements
        .try_reserve(remaining)
        .map_err(|_| Error::Allocation)?;
    observed
        .items
        .try_reserve(remaining)
        .map_err(|_| Error::Allocation)?;
    let events = &mut receipt.events[input.next..];
    let mut tail = control
        .submit_surface_click_batch(&input.owner, input.revision, events)
        .map_err(Error::Batch)?;
    // 内层提交保留逐项 ACK；观察异常独立处理，不覆盖已发送证据或原前缀。
    let observation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        observation::observe(control, events, &mut tail)
    }))
    .unwrap_or(Err(Error::Panicked));
    match observation {
        Ok(mut tail_observed) => {
            observed.items.append(&mut tail_observed.items);
            if let Some(snapshot) = tail_observed.snapshot {
                if observed
                    .snapshot
                    .as_ref()
                    .is_none_or(|current| current.revision <= snapshot.revision)
                {
                    observed.snapshot = Some(snapshot);
                }
            }
        }
        Err(error) => {
            // 每个已收到的 ACK 都保留一项失败观察，不伪造版本或擦除前缀。
            observed
                .items
                .extend((0..tail.acknowledgements.len()).map(|_| ObservedItem {
                    decision: None,
                    result: Err(ObservationError::Unavailable(error)),
                    commit: None,
                    confirmation: None,
                }));
        }
    }
    submitted.stop = tail.stop;
    submitted
        .acknowledgements
        .append(&mut tail.acknowledgements);
    Ok(())
}
impl Outcome {
    /// 结果已在完整收据内更新；外层错误单独返回供应用通知，原证据不被替换。
    pub fn into_batch(self) -> (super::Outcome, Result<(), Error>) {
        let Input {
            owner,
            revision,
            receipt,
            ..
        } = self.input;
        (
            super::Outcome {
                input: super::Input {
                    owner,
                    revision,
                    events: receipt.events,
                },
                submission: receipt.submission,
                observation: receipt.observation,
                delivery: self.delivery,
            },
            self.result,
        )
    }
}
