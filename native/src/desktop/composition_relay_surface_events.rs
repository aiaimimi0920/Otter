//! 应用 Surface 事件任务集合：完成缓存仍占容量，拒收/取消不得丢失事件证据。
use super::*;
use crate::{
    desktop::surface_authorized_window::EventReceipt,
    surface_click_event::ClickEvent,
    surface_event_runtime::{
        Error as EventError, Job as EventJob, Outcome as EventOutcome, StartError,
    },
    surface_lifecycle_owner::Owner,
};
// 收据独立于网络槽位，不能堵住附件停用；尚未持久化的动作最多保留 32 个。
pub(super) const MAX_EVENT_RECEIPTS: usize = 32;
impl Sessions {
    pub(super) fn event_backlog(&self) -> usize {
        self.event_receipts.len()
            + self.batch_event_backlog()
            + self.event_jobs.len()
            + self.event_outcomes.len()
            + self.decision_jobs.len()
    }
    pub fn start_surface_event(
        &mut self,
        config: Config,
        lease: Lease,
        owner: Owner,
        revision: u64,
        event: ClickEvent,
    ) -> Result<(), StartError> {
        let error = if self.closed {
            Some(io::ErrorKind::BrokenPipe.into())
        } else if self.occupied() >= MAX_SESSIONS || self.event_backlog() >= MAX_EVENT_RECEIPTS {
            Some(io::ErrorKind::WouldBlock.into())
        } else {
            self.event_jobs.try_reserve(1).err().map(io::Error::other)
        };
        if let Some(error) = error {
            return Err(StartError {
                owner: Box::new(owner),
                event: Box::new(event),
                error,
            });
        }
        // 为所有在途任务的完成缓存预留空间，回收后不能因分配失败丢失 owner。
        if let Err(error) = self
            .event_outcomes
            .try_reserve(self.event_jobs.len() + self.decision_jobs.len() + 1)
        {
            return Err(StartError {
                owner: Box::new(owner),
                event: Box::new(event),
                error: io::Error::other(error),
            });
        }
        self.event_jobs
            .push(EventJob::start(config, lease, owner, revision, event)?);
        Ok(())
    }
    pub(super) fn cancel_surface_events(&mut self) {
        for job in &self.event_jobs {
            job.request_stop();
        }
        // 已收割结果保留 ACK/失败，只撤销交付资格。
        for outcome in &mut self.event_outcomes {
            outcome.delivery = Err(crate::device_authorization_lease::Error::Revoked);
        }
    }
    pub(super) fn poll_surface_events(&mut self) -> Vec<(String, surfaces::Error)> {
        // 时钟失败采用失效时刻，绝不凭时间 0 延长授权；没有周期定时器。
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.event_jobs.len() {
            if let Some(outcome) = self.event_jobs[index].try_join(now) {
                self.event_jobs.swap_remove(index);
                let error = outcome
                    .result
                    .as_ref()
                    .err()
                    .or_else(|| {
                        outcome
                            .confirmation
                            .as_ref()
                            .and_then(|result| result.as_ref().err())
                    })
                    .or_else(|| {
                        outcome
                            .convergence
                            .as_ref()
                            .and_then(|result| result.as_ref().err())
                    });
                if let Some(error) = error {
                    if !matches!(error, EventError::Stopped | EventError::Convergence(
                        crate::live_relay_http::ConvergenceError::Stopped
                        | crate::live_relay_http::ConvergenceError::ConfirmationRequired
                    ) | EventError::Confirmation(crate::live_relay_http::ConfirmationError::Stopped)) {
                        errors.push((
                            outcome.owner.binding().origin().to_owned(),
                            surfaces::Error::Event(*error),
                        ));
                    }
                }
                self.event_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    /// 返回 false 时原结果仍留在集合；旧附件的拒收结果不阻塞后面的匹配结果。
    pub fn deliver_surface_event(
        &mut self,
        mut deliver: impl FnMut(EventOutcome) -> Result<(), Box<EventOutcome>>,
    ) -> bool {
        for index in (0..self.event_outcomes.len()).rev() {
            let outcome = self.event_outcomes.remove(index);
            match deliver(outcome) {
                Ok(()) => return true,
                Err(outcome) => self.event_outcomes.insert(index, *outcome),
            }
        }
        false
    }
    pub fn take_surface_event_outcome(&mut self) -> Option<EventOutcome> {
        self.event_outcomes.pop()
    }
    /// 在调用窗口之前预留收据内存；拒收保留完整结果，不阻塞其他匹配事件。
    pub fn settle_surface_events(
        &mut self,
        mut deliver: impl FnMut(EventOutcome) -> Result<EventReceipt, Box<EventOutcome>>,
    ) -> io::Result<usize> {
        let available = MAX_EVENT_RECEIPTS.saturating_sub(self.event_receipts.len());
        self.event_receipts
            .try_reserve(available.min(self.event_outcomes.len()))
            .map_err(io::Error::other)?;
        let mut accepted = 0;
        for index in (0..self.event_outcomes.len()).rev() {
            if accepted == available {
                break;
            }
            let outcome = self.event_outcomes.remove(index);
            match deliver(outcome) {
                Ok(receipt) => {
                    self.event_receipts.push(receipt);
                    accepted += 1;
                }
                Err(outcome) => self.event_outcomes.insert(index, *outcome),
            }
        }
        Ok(accepted)
    }
    /// 显式转移证据所有权；调用方必须负责后续保存，不能按普通 UI 通知丢弃。
    pub fn take_surface_event_receipt(&mut self) -> Option<EventReceipt> {
        self.event_receipts.pop()
    }
}
