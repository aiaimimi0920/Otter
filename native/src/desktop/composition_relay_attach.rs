//! Initial-attachment collection: cancellation only prevents scene delivery, preserving recovery context for confirmed bindings or unknown creations.
use super::*;
use crate::surface_attach_runtime::{Error, Job, Outcome, Request};

impl Sessions {
    pub fn attaching(&self) -> bool {
        !self.attach_jobs.is_empty()
    }
    /// Return unchanged when delivery capacity is insufficient; pop must not lose evidence of unknown creation or confirmed binding.
    pub fn deliver_attach(
        &mut self,
        deliver: impl FnOnce(Outcome) -> Result<(), Box<Outcome>>,
    ) -> bool {
        let Some(outcome) = self.attach_outcomes.pop() else {
            return false;
        };
        match deliver(outcome) {
            Ok(()) => true,
            Err(outcome) => {
                self.attach_outcomes.push(*outcome);
                false
            }
        }
    }
    pub fn start_attach(
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
        self.attach_jobs.try_reserve(1).map_err(io::Error::other)?;
        // Reserve result capacity before reclamation; uncollected results still count toward total quota and cannot accumulate unboundedly.
        self.attach_outcomes
            .try_reserve(self.attach_jobs.len() + 1)
            .map_err(io::Error::other)?;
        self.attach_jobs.push(Job::start(config, lease, request)?);
        Ok(())
    }
    pub(super) fn cancel_attach_requests(&mut self) {
        for job in &self.attach_jobs {
            job.request_stop();
        }
        for outcome in &mut self.attach_outcomes {
            outcome.result = Err(Error::Stopped);
        }
    }
    pub(super) fn poll_attaches(&mut self) -> Vec<(String, super::surfaces::Error)> {
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.attach_jobs.len() {
            if let Some(outcome) = self.attach_jobs[index].try_join() {
                self.attach_jobs.swap_remove(index);
                if let Err(error) = &outcome.result {
                    if *error != Error::Stopped {
                        errors.push((
                            outcome.origin.clone(),
                            super::surfaces::Error::Attach(*error),
                        ));
                    }
                }
                self.attach_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    /// Transfer only recovery evidence; callers must revalidate authorization and never automatically replay unknown creation.
    pub fn take_attach_outcome(&mut self) -> Option<Outcome> {
        self.attach_outcomes.pop()
    }
}
