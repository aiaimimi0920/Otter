//! 接续任务共用批次交付与预算；前缀证据继续占预算，不因只发送后缀而漏算。
use super::*;
use crate::surface_event_batch_runtime::{
    continuation::{Input, Job, StartError},
    Error,
};
impl Sessions {
    pub(super) fn reserve_batch_outcomes(&mut self) -> io::Result<()> {
        self.batch_outcomes
            .try_reserve(
                self.batch_jobs.len()
                    + self.batch_decision_jobs.len()
                    + self.batch_continuation_jobs.len()
                    + 1,
            )
            .map_err(io::Error::other)
    }
    pub fn start_surface_batch_continuation(
        &mut self,
        config: Config,
        lease: Lease,
        input: Input,
    ) -> Result<(), StartError> {
        let count = input.receipt.events.len();
        let preparation = (|| {
            if self.closed {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            if input.receipt.continuation_boundary() != Some(input.next)
                || input.revision != input.receipt.revision
            {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            if self.occupied() >= MAX_SESSIONS
                || count > surface_events::MAX_EVENT_RECEIPTS.saturating_sub(self.event_backlog())
            {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            self.batch_continuation_jobs
                .try_reserve(1)
                .map_err(io::Error::other)?;
            self.reserve_batch_outcomes()
        })();
        if let Err(error) = preparation {
            return Err(StartError {
                input: Box::new(input),
                error,
            });
        }
        self.batch_continuation_jobs
            .push((count, Job::start(config, lease, input)?));
        Ok(())
    }
    pub(super) fn cancel_batch_continuations(&mut self) {
        for (_, job) in &self.batch_continuation_jobs {
            job.request_stop();
        }
    }
    pub(super) fn poll_batch_continuations(&mut self) -> Vec<(String, surfaces::Error)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .unwrap_or(u64::MAX);
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.batch_continuation_jobs.len() {
            if let Some(outcome) = self.batch_continuation_jobs[index].1.try_join(now) {
                self.batch_continuation_jobs.swap_remove(index);
                let (outcome, result) = outcome.into_batch();
                if let Err(error) = result {
                    if error != Error::Stopped {
                        errors.push((
                            outcome.input.owner.binding().origin().to_owned(),
                            surfaces::Error::Batch(error),
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
}
