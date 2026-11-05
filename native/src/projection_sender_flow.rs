//! 原网络 Job 内的当前帧准备与请求交接；先登记持久意图，任何结果过滤都不能丢取消责任。
use super::*;
use crate::projection_delivery::sender::{binding::Candidate, Request};
use std::sync::Mutex;

pub(crate) struct SenderInput {
    pub request: Request,
    pub source: crate::session_source::Source,
    pub name: String,
}
pub(crate) type SenderPreparation =
    Box<dyn FnOnce(&Lease, &dyn Fn(Request)) -> Result<SenderInput, Error> + Send + 'static>;
#[derive(Default)]
pub(super) struct Attempt(Mutex<Option<Request>>);
impl Attempt {
    pub(super) fn record(&self, request: Request) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(request);
    }
    pub(super) fn take(&self) -> Option<Request> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}
impl Job {
    pub(crate) fn start_sender(
        config: Config,
        lease: Lease,
        prepare: SenderPreparation,
    ) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::Sender(prepare),
            Some(Slot::acquire()?),
            None,
        )
    }
    /// 只移动内存快照；必须在完成回收后、closed/discarded UI 过滤前领取。
    pub(crate) fn take_sender_request(&mut self) -> Option<Request> {
        self.thread
            .is_none()
            .then(|| self.sender_attempt.take())
            .flatten()
    }
}
pub(super) fn run(
    prepare: SenderPreparation,
    control: &crate::live_relay_http::Control<'_>,
    task: &Arc<Lease>,
    attempt: &Attempt,
) -> Result<Completed, Error> {
    let input = prepare(task, &|request| attempt.record(request))?;
    // prepare 返回后尚未 activate/HTTP；撤权也仍登记 Request，供后台减权接管。
    attempt.record(input.request.clone());
    let mut connected = send(&input.request, control, task)?;
    let candidate = crate::projection_delivery::sender::binding::Binding::prepare(
        &input.source,
        input.request.intent(),
        &connected.response,
        &input.name,
        task.preparation_cancel(),
    )
    .map_err(Error::Sender)?;
    connected.candidate = Some(candidate);
    Ok(Completed::Sent(Box::new(connected)))
}
pub(super) fn send(
    request: &Request,
    control: &crate::live_relay_http::Control<'_>,
    task: &Arc<Lease>,
) -> Result<sent::Connected, Error> {
    let active = request
        .activate(task.clone(), false)
        .map_err(Error::Sender)?;
    let result = control.send_projection(&active);
    if task.is_revoked() {
        request.cancel().map_err(Error::Sender)?;
        return Err(Error::Stopped);
    }
    let response = result.map_err(Error::Http)?;
    if response.editing().is_some() {
        return Err(Error::Sender(
            crate::projection_delivery::sender::Error::Changed,
        ));
    }
    Ok(sent::Connected {
        active,
        response,
        candidate: None,
    })
}
impl SenderReady {
    /// UI 只核租约代次/时效，不进行 outbox 磁盘基线 I/O；候选已由原 worker 完整核验。
    pub(crate) fn authorized(&self, current: &Lease, now: u64) -> bool {
        let lease = self.value.active.lease();
        lease.same_grant(current)
            && lease
                .validate(self.request().intent().envelope().origin(), now)
                .is_ok()
            && current
                .validate(self.request().intent().envelope().origin(), now)
                .is_ok()
    }
    pub(crate) fn take_binding(&mut self) -> Option<Candidate> {
        self.value.candidate.take()
    }
}
