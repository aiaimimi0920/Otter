//! 批次决定应用任务：共享容量，收割后并回批次；异常输入保留且阻止遗漏退出日志。
use super::*;
use crate::surface_batch_decision_runtime::{Error, Input, Job, Outcome, StartError};
impl Sessions {
    pub fn start_surface_batch_decision(
        &mut self,
        config: Config,
        lease: Lease,
        input: Input,
    ) -> Result<(), StartError> {
        let count = input.prepared.receipt.events.len();
        let preparation = (|| {
            if self.closed {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            if count == 0 {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            if self.occupied() >= MAX_SESSIONS
                || count > surface_events::MAX_EVENT_RECEIPTS.saturating_sub(self.event_backlog())
            {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            self.batch_decision_jobs
                .try_reserve(1)
                .map_err(io::Error::other)?;
            // 所有批次类任务都可能在下一次收割前完成。
            self.reserve_batch_outcomes()?;
            self.batch_decision_rejected
                .try_reserve(self.batch_decision_jobs.len() + 1)
                .map_err(io::Error::other)
        })();
        if let Err(error) = preparation {
            return Err(StartError {
                input: Box::new(input),
                error,
            });
        }
        self.batch_decision_jobs
            .push((count, Job::start(config, lease, input)?));
        Ok(())
    }
    pub(super) fn cancel_batch_decisions(&mut self) {
        for (_, job) in &self.batch_decision_jobs {
            job.request_stop();
        }
        for outcome in &mut self.batch_decision_rejected {
            outcome.delivery = Err(crate::device_authorization_lease::Error::Revoked);
        }
    }
    pub(super) fn poll_batch_decisions(&mut self) -> Vec<(String, surfaces::Error)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.batch_decision_jobs.len() {
            if let Some(outcome) = self.batch_decision_jobs[index].1.try_join(now) {
                self.batch_decision_jobs.swap_remove(index);
                let origin = outcome.input.owner.binding().origin().to_owned();
                if let Err(error) = outcome.submission {
                    if error != Error::Stopped {
                        errors.push((origin.clone(), surfaces::Error::Decision(error)));
                    }
                }
                match outcome.into_batch() {
                    Ok(outcome) => self.batch_outcomes.push(outcome),
                    Err(outcome) => {
                        errors.push((origin, surfaces::Error::BatchDecisionMerge));
                        self.batch_decision_rejected.push(*outcome);
                    }
                }
            } else {
                index += 1;
            }
        }
        errors
    }
    /// 显式移交异常输入的保存责任；不能静默丢弃，亦不能自动重试决定。
    pub fn take_rejected_batch_decision(&mut self) -> Option<Outcome> {
        self.batch_decision_rejected.pop()
    }
}
