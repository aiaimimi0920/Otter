//! 恢复查询与现有挂载共用集合容量；候选只供显式确认，不自动发起 remount。
use super::*;
use crate::{
    surface_attach_runtime::Request,
    surface_recovery_runtime::{Error, Job, Outcome},
};
impl Sessions {
    pub fn query_surface_recovery(
        &mut self,
        config: Config,
        lease: Lease,
        request: Arc<Request>,
    ) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.recovery_jobs
            .try_reserve(1)
            .map_err(io::Error::other)?;
        self.recovery_outcomes
            .try_reserve(self.recovery_jobs.len() + 1)
            .map_err(io::Error::other)?;
        self.recovery_jobs.push(Job::start(config, lease, request)?);
        Ok(())
    }
    pub(super) fn cancel_recovery_requests(&mut self) {
        for job in &self.recovery_jobs {
            job.request_stop();
        }
        for outcome in &mut self.recovery_outcomes {
            outcome.result = Err(Error::Stopped);
        }
    }
    pub(super) fn poll_recoveries(&mut self) -> Vec<(String, super::surfaces::Error)> {
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.recovery_jobs.len() {
            if let Some(outcome) = self.recovery_jobs[index].try_join() {
                self.recovery_jobs.swap_remove(index);
                if let Err(error) = &outcome.result {
                    if *error != Error::Stopped {
                        errors.push((
                            outcome.origin.clone(),
                            super::surfaces::Error::Recovery(*error),
                        ));
                    }
                }
                self.recovery_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    pub fn take_recovery_outcome(&mut self) -> Option<Outcome> {
        self.recovery_outcomes.pop()
    }
}
