//! 批次应用集合：以事件数计未保存预算，完成后保留 owner，收据不占网络槽位。
use super::*;
use crate::{
    desktop::surface_authorized_window::BatchReceipt,
    surface_event_batch_runtime::{
        Error as BatchError, Input, Job as BatchJob, Outcome, StartError,
    },
};
impl Sessions {
    pub(super) fn batch_event_backlog(&self) -> usize {
        self.batch_jobs
            .iter()
            .map(|(count, _)| count)
            .sum::<usize>()
            + self
                .batch_continuation_jobs
                .iter()
                .map(|(count, _)| count)
                .sum::<usize>()
            + self
                .batch_decision_jobs
                .iter()
                .map(|(count, _)| count)
                .sum::<usize>()
            + self
                .batch_decision_rejected
                .iter()
                .map(|outcome| outcome.input.prepared.receipt.events.len())
                .sum::<usize>()
            + self
                .batch_outcomes
                .iter()
                .map(|outcome| outcome.input.events.len())
                .sum::<usize>()
            + self
                .batch_receipts
                .iter()
                .map(|receipt| receipt.events.len())
                .sum::<usize>()
    }
    pub fn start_surface_batch(
        &mut self,
        config: Config,
        lease: Lease,
        input: Input,
    ) -> Result<(), StartError> {
        let count = input.events.len();
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
            self.batch_jobs.try_reserve(1).map_err(io::Error::other)?;
            self.reserve_batch_outcomes()
        })();
        if let Err(error) = preparation {
            return Err(StartError {
                input: Box::new(input),
                error,
            });
        }
        self.batch_jobs
            .push((count, BatchJob::start(config, lease, input)?));
        Ok(())
    }
    pub(super) fn cancel_surface_batches(&mut self) {
        for (_, job) in &self.batch_jobs {
            job.request_stop();
        }
        for outcome in &mut self.batch_outcomes {
            outcome.delivery = Err(crate::device_authorization_lease::Error::Revoked);
        }
    }
    pub(super) fn poll_surface_batches(&mut self) -> Vec<(String, surfaces::Error)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.batch_jobs.len() {
            if let Some(outcome) = self.batch_jobs[index].1.try_join(now) {
                self.batch_jobs.swap_remove(index);
                if let Err(error) = &outcome.submission {
                    if *error != BatchError::Stopped {
                        errors.push((
                            outcome.input.owner.binding().origin().to_owned(),
                            surfaces::Error::Batch(*error),
                        ));
                    }
                }
                self.batch_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    /// 分配发生在窗口副作用之前；拒收原样保留，不阻塞其他附件的可接纳结果。
    pub fn settle_surface_batches(
        &mut self,
        mut deliver: impl FnMut(Outcome) -> Result<BatchReceipt, Box<Outcome>>,
    ) -> io::Result<usize> {
        self.batch_receipts
            .try_reserve(self.batch_outcomes.len())
            .map_err(io::Error::other)?;
        let mut accepted = 0;
        for index in (0..self.batch_outcomes.len()).rev() {
            let outcome = self.batch_outcomes.remove(index);
            match deliver(outcome) {
                Ok(receipt) => {
                    self.batch_receipts.push(receipt);
                    accepted += 1;
                }
                Err(outcome) => self.batch_outcomes.insert(index, *outcome),
            }
        }
        Ok(accepted)
    }
    pub fn take_surface_batch_outcome(&mut self) -> Option<Outcome> {
        self.batch_outcomes.pop()
    }
    pub fn take_surface_batch_receipt(&mut self) -> Option<BatchReceipt> {
        self.batch_receipts.pop()
    }
}
