//! 应用决定任务集合：与点击任务共用容量/退出泵，完成证据保留至显式交付。
use super::*;
use crate::surface_decision_runtime::{
    Error as DecisionError, Input, Job as DecisionJob, StartError,
};
pub enum ConfirmationAction {
    /// 取消或 UI 准备失败必须完整返还原收据，不能按通知直接丢弃。
    Keep(crate::desktop::surface_authorized_window::EventReceipt),
    Start(Config, Lease, Input),
}
impl Sessions {
    pub fn pending_surface_confirmation(&self, origin: &str, now_ms: u64) -> Option<&str> {
        self.pending_surface_confirmation_for(origin, now_ms, |_| true)
    }
    /// 应用在同一次选择中复核窗口；无可用 HWND 的旧收据不能挡住其他附件。
    pub fn pending_surface_confirmation_for(
        &self,
        origin: &str,
        now_ms: u64,
        mut available: impl FnMut(&crate::desktop::surface_authorized_window::EventReceipt) -> bool,
    ) -> Option<&str> {
        self.event_receipts
            .iter()
            .find(|receipt| {
                receipt.event.binding().origin() == origin
                    && receipt.confirmation_ready(now_ms)
                    && available(receipt)
            })
            .map(|receipt| receipt.event.id())
    }
    /// 只处理明确选中的事件；不轮询弹窗、不自动决定。调用方不能在回调中遗失证据。
    pub fn dispatch_surface_confirmation(
        &mut self,
        event_id: &str,
        prepare: impl FnOnce(
            crate::desktop::surface_authorized_window::EventReceipt,
        ) -> ConfirmationAction,
    ) -> io::Result<bool> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_millis();
        let now = u64::try_from(now).map_err(io::Error::other)?;
        let Some(index) = self
            .event_receipts
            .iter()
            .position(|receipt| receipt.event.id() == event_id && receipt.confirmation_ready(now))
        else {
            return Ok(false);
        };
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        // 在 UI 消费提示和窗口移交 owner 之前保证失败结果有存放空间。
        self.event_outcomes
            .try_reserve(self.event_jobs.len() + self.decision_jobs.len() + 1)
            .map_err(io::Error::other)?;
        let receipt = self.event_receipts.remove(index);
        match prepare(receipt) {
            ConfirmationAction::Keep(receipt) => {
                self.event_receipts.insert(index, receipt);
                Ok(false)
            }
            ConfirmationAction::Start(config, lease, input) => {
                match self.start_surface_decision(config, lease, input) {
                    Ok(()) => Ok(true),
                    Err(failure) => {
                        // 不重发决定；主循环与退出泵负责将 owner 和证据结算回窗口。
                        self.event_outcomes.push(
                            crate::surface_decision_runtime::Outcome {
                                input: *failure.input,
                                submission: Err(DecisionError::NotStarted),
                                convergence: None,
                                delivery: Ok(()),
                            }
                            .into_event(),
                        );
                        Err(failure.error)
                    }
                }
            }
        }
    }
    pub fn start_surface_decision(
        &mut self,
        config: Config,
        lease: Lease,
        input: Input,
    ) -> Result<(), StartError> {
        let error = if self.closed {
            Some(io::ErrorKind::BrokenPipe.into())
        } else if self.occupied() >= MAX_SESSIONS
            || self.event_backlog() >= surface_events::MAX_EVENT_RECEIPTS
        {
            Some(io::ErrorKind::WouldBlock.into())
        } else {
            self.decision_jobs
                .try_reserve(1)
                .err()
                .map(io::Error::other)
        };
        if let Some(error) = error {
            return Err(StartError {
                input: Box::new(input),
                error,
            });
        }
        // 完成前预留缓存，不能在释放线程槽位后因分配失败丢失决定和 owner。
        if let Err(error) = self
            .event_outcomes
            .try_reserve(self.decision_jobs.len() + self.event_jobs.len() + 1)
        {
            return Err(StartError {
                input: Box::new(input),
                error: io::Error::other(error),
            });
        }
        self.decision_jobs
            .push(DecisionJob::start(config, lease, input)?);
        Ok(())
    }
    pub(super) fn cancel_surface_decisions(&mut self) {
        for job in &self.decision_jobs {
            job.request_stop();
        }
    }
    pub(super) fn poll_surface_decisions(&mut self) -> Vec<(String, surfaces::Error)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.decision_jobs.len() {
            if let Some(outcome) = self.decision_jobs[index].try_join(now) {
                self.decision_jobs.swap_remove(index);
                let error = outcome.submission.as_ref().err().or_else(|| {
                    outcome
                        .convergence
                        .as_ref()
                        .and_then(|value| value.as_ref().err())
                });
                let cancelled = outcome.submission.is_ok()
                    && outcome.input.ack.status() == crate::surface_action_ack::Status::Cancelled;
                if let Some(error) = error {
                    if !cancelled
                        && !matches!(
                            error,
                            DecisionError::Stopped
                                | DecisionError::Convergence(
                                    crate::live_relay_http::ConvergenceError::Stopped
                                )
                                | DecisionError::Submit(
                                    crate::live_relay_http::DecisionSubmitError::Stopped
                                )
                        )
                    {
                        errors.push((
                            outcome.input.owner.binding().origin().to_owned(),
                            surfaces::Error::Decision(*error),
                        ));
                    }
                }
                self.event_outcomes.push(outcome.into_event());
            } else {
                index += 1;
            }
        }
        errors
    }
}
