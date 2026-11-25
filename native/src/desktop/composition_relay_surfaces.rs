//! Surface transactions in remote collections; completed results retain owners and capacity until explicitly collected by the UI.
use super::*;
use crate::{
    surface_lifecycle_owner::Owner,
    surface_lifecycle_runtime::{
        Action, Error as LifecycleError, Job as SurfaceJob, Outcome as SurfaceOutcome, StartError,
    },
};
#[derive(Debug)]
pub enum Error {
    Lifecycle(LifecycleError),
    Resource(crate::surface_resource_runtime::Error),
    Attach(crate::surface_attach_runtime::Error),
    Recovery(crate::surface_recovery_runtime::Error),
    Event(crate::surface_event_runtime::Error),
    Decision(crate::surface_decision_runtime::Error),
    Batch(crate::surface_event_batch_runtime::Error),
    BatchDecisionMerge,
}
impl Sessions {
    /// Snapshot undelivered evidence after normal reclamation; reject fabricated complete snapshots while still in flight.
    pub fn surface_recovery_records(
        &self,
    ) -> io::Result<Vec<crate::surface_recovery_record::Record>> {
        if !self.event_outcomes.is_empty()
            || !self.event_receipts.is_empty()
            || self.batch_event_backlog() != 0
        {
            return Err(io::Error::other(
                "Surface recovery snapshot has unsaved event evidence",
            ));
        }
        self.attachment_recovery_records()
    }
    /// Snapshot both recovery-hint categories together; copy only sanitized metadata without prematurely taking/clearing in-memory evidence.
    pub fn surface_recovery_bundle(
        &self,
    ) -> io::Result<(
        Vec<crate::surface_recovery_record::Record>,
        Vec<crate::surface_event_record::Record>,
    )> {
        let attachments = self.attachment_recovery_records()?;
        let mut events: Vec<_> = self
            .event_outcomes
            .iter()
            .map(crate::surface_event_record::Record::from_outcome)
            .chain(
                self.event_receipts
                    .iter()
                    .map(crate::surface_event_record::Record::from_receipt),
            )
            .collect();
        for outcome in &self.batch_outcomes {
            events.extend(crate::surface_event_record::Record::from_batch_outcome(
                outcome,
            )?);
        }
        for receipt in &self.batch_receipts {
            events.extend(crate::surface_event_record::Record::from_batch_receipt(
                receipt,
            )?);
        }
        Ok((attachments, events))
    }
    fn attachment_recovery_records(
        &self,
    ) -> io::Result<Vec<crate::surface_recovery_record::Record>> {
        use crate::surface_recovery_record::Record;
        if !self.surface_jobs.is_empty()
            || !self.event_jobs.is_empty()
            || !self.decision_jobs.is_empty()
            || !self.batch_jobs.is_empty()
            || !self.batch_decision_jobs.is_empty()
            || !self.batch_continuation_jobs.is_empty()
            || !self.batch_decision_rejected.is_empty()
            || !self.attach_jobs.is_empty()
            || !self.recovery_jobs.is_empty()
        {
            return Err(io::Error::other(
                "Surface recovery snapshot has running jobs or unsaved event evidence",
            ));
        }
        let mut records = Vec::new();
        for outcome in &self.batch_outcomes {
            records.push(Record::from_binding(
                outcome.input.owner.binding(),
                None,
                outcome.input.owner.uncertain(),
                false,
            ));
        }
        for outcome in &self.event_outcomes {
            records.push(Record::from_binding(
                outcome.owner.binding(),
                None,
                outcome.owner.uncertain(),
                false,
            ));
        }
        for outcome in &self.surface_outcomes {
            records.push(Record::from_binding(
                outcome.owner.binding(),
                None,
                outcome.owner.uncertain(),
                false,
            ));
        }
        for outcome in &self.attach_outcomes {
            records.push(match &outcome.binding {
                Some(binding) => Record::from_binding(
                    binding,
                    Some(outcome.request.art()),
                    outcome.uncertain,
                    false,
                ),
                None => Record::from_request(
                    &outcome.origin,
                    &outcome.device,
                    &outcome.request,
                    outcome.uncertain,
                ),
            });
        }
        for outcome in &self.recovery_outcomes {
            records.push(Record::from_request(
                &outcome.origin,
                &outcome.device,
                &outcome.request,
                true,
            ));
        }
        Ok(records)
    }
    /// Graceful shutdown waits for mount/lifecycle transactions and must not repeatedly wake for unrelated frame events or unconsumed results.
    pub fn surface_shutdown_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.surface_jobs
            .iter()
            .filter_map(SurfaceJob::wait_handle)
            .chain(
                self.resource_jobs
                    .iter()
                    .filter_map(crate::surface_resource_runtime::Job::wait_handle),
            )
            .chain(
                self.batch_continuation_jobs
                    .iter()
                    .filter_map(|(_, job)| job.wait_handle()),
            )
            .chain(
                self.batch_decision_jobs
                    .iter()
                    .filter_map(|(_, job)| job.wait_handle()),
            )
            .chain(
                self.batch_jobs
                    .iter()
                    .filter_map(|(_, job)| job.wait_handle()),
            )
            .chain(
                self.decision_jobs
                    .iter()
                    .filter_map(crate::surface_decision_runtime::Job::wait_handle),
            )
            .chain(
                self.event_jobs
                    .iter()
                    .filter_map(crate::surface_event_runtime::Job::wait_handle),
            )
            .chain(
                self.attach_jobs
                    .iter()
                    .filter_map(crate::surface_attach_runtime::Job::wait_handle),
            )
    }
    /// Thread reclamation is not attachment settlement; unadmitted results must remain as pending evidence.
    pub fn surface_cleanup_pending(&self) -> bool {
        !self.batch_continuation_jobs.is_empty()
            || !self.resource_jobs.is_empty()
            || !self.surface_jobs.is_empty()
            || !self.batch_jobs.is_empty()
            || !self.batch_decision_jobs.is_empty()
            || !self.batch_decision_rejected.is_empty()
            || !self.batch_outcomes.is_empty()
            || !self.batch_receipts.is_empty()
            || !self.decision_jobs.is_empty()
            || !self.event_jobs.is_empty()
            || !self.event_outcomes.is_empty()
            || !self.event_receipts.is_empty()
            || !self.surface_outcomes.is_empty()
            || !self.attach_jobs.is_empty()
            || !self.attach_outcomes.is_empty()
            || !self.recovery_jobs.is_empty()
            || !self.recovery_outcomes.is_empty()
    }
    pub fn deliver_surface(
        &mut self,
        mut deliver: impl FnMut(SurfaceOutcome) -> Result<(), Box<SurfaceOutcome>>,
    ) -> bool {
        // Bounded scan: unmatched recovery results must not block later activation completions; retain rejected entries in place.
        for index in (0..self.surface_outcomes.len()).rev() {
            let outcome = self.surface_outcomes.remove(index);
            match deliver(outcome) {
                Ok(()) => return true,
                Err(outcome) => self.surface_outcomes.insert(index, *outcome),
            }
        }
        false
    }
    pub(super) fn occupied(&self) -> usize {
        self.entries.len()
            + self.sources.len()
            + usize::from(self.recovery_file_job.is_some())
            + usize::from(self.recovery_file.is_some())
            + usize::from(self.catalog_job.is_some())
            + usize::from(self.projection_job.is_some())
            + self.surface_jobs.len()
            + self.surface_outcomes.len()
            + self.resource_jobs.len()
            + self.resource_ready.len()
            + self.event_jobs.len()
            + self.event_outcomes.len()
            + self.decision_jobs.len()
            + self.batch_jobs.len()
            + self.batch_decision_jobs.len()
            + self.batch_continuation_jobs.len()
            + self.batch_decision_rejected.len()
            + self.batch_outcomes.len()
            + self.attach_jobs.len()
            + self.attach_outcomes.len()
            + self.recovery_jobs.len()
            + self.recovery_outcomes.len()
    }
    pub(in crate::desktop) fn has_surface_worker_slot(&self) -> bool {
        !self.closed && self.occupied() < MAX_SESSIONS
    }
    pub fn start_surface(
        &mut self,
        config: Config,
        lease: Lease,
        owner: Owner,
        action: Action,
    ) -> Result<(), StartError> {
        let error = if self.closed {
            Some(io::ErrorKind::BrokenPipe.into())
        } else if !self.has_surface_worker_slot() {
            Some(io::ErrorKind::WouldBlock.into())
        } else {
            self.surface_jobs.try_reserve(1).err().map(io::Error::other)
        };
        if let Some(error) = error {
            return Err(StartError {
                owner: Box::new(owner),
                error,
            });
        }
        // Reserve completion caches early to prevent allocation failure after thread reclamation from losing owners.
        if let Err(error) = self
            .surface_outcomes
            .try_reserve(self.surface_jobs.len() + 1)
        {
            return Err(StartError {
                owner: Box::new(owner),
                error: io::Error::other(error),
            });
        }
        self.surface_jobs
            .push(SurfaceJob::start(config, lease, owner, action)?);
        Ok(())
    }
    pub fn cancel_surface_requests(&mut self) {
        self.cancel_surface_resources(|_, _| true);
        self.cancel_batch_continuations();
        self.cancel_batch_decisions();
        self.cancel_surface_batches();
        self.cancel_surface_decisions();
        self.cancel_surface_events();
        self.cancel_recovery_file();
        self.cancel_recovery_requests();
        self.cancel_attach_requests();
        for job in &self.surface_jobs {
            job.request_stop();
        }
        // Do not deliver old scenes after service switches/exit, but preserve the owner's confirmed revision or unknown state.
        for outcome in &mut self.surface_outcomes {
            outcome.result = Err(LifecycleError::Stopped);
        }
    }
    /// Return a one-shot error notice; the collection retains the result's owner, which the notice does not consume.
    pub fn poll_surfaces(&mut self) -> Vec<(String, Error)> {
        self.poll_recovery_file();
        let mut errors = self.poll_attaches();
        errors.extend(self.poll_surface_resources());
        errors.extend(self.poll_recoveries());
        errors.extend(self.poll_surface_events());
        errors.extend(self.poll_surface_decisions());
        errors.extend(self.poll_surface_batches());
        errors.extend(self.poll_batch_decisions());
        errors.extend(self.poll_batch_continuations());
        let mut index = 0;
        while index < self.surface_jobs.len() {
            if let Some(outcome) = self.surface_jobs[index].try_join() {
                self.surface_jobs.swap_remove(index);
                if let Err(error) = &outcome.result {
                    if *error != LifecycleError::Stopped {
                        errors.push((
                            outcome.owner.binding().origin().to_owned(),
                            Error::Lifecycle(*error),
                        ));
                    }
                }
                self.surface_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    pub fn take_surface_outcome(&mut self) -> Option<SurfaceOutcome> {
        self.surface_outcomes.pop()
    }
}
