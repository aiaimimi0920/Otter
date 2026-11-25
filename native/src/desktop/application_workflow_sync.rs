//! UI sync owner; Receipt stays on the UI thread, while background work holds only fixed connection identity and prepared data.
use super::*;
use crate::desktop::session_candidate::{WorkflowDelivery, WorkflowSyncJob};
#[path = "application_workflow_completion.rs"]
pub(in crate::desktop::application) mod completion;

pub(in crate::desktop::application) struct Sync {
    receipt: Receipt,
    job: WorkflowSyncJob,
}
impl Candidate {
    pub(in crate::desktop::application) unsafe fn start_sync(
        self,
        app: &Application,
    ) -> Result<Sync> {
        app.require_workflow_slot()?;
        if !self.matches(app) {
            return Err(Error::new(E_ABORT, "Workflow sync candidate became stale"));
        }
        let identity = self
            .receipt
            .catalog
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    E_ABORT,
                    "Workflow sync requires admitted capability identity",
                )
            })?
            .identity
            .clone();
        let limit = crate::session_document::MAX_SESSION_BYTES;
        // At most one encoded request and one transport copy; do not build message queues proportional to node count.
        let budget = crate::byte_budget::Budget::new(limit * 2);
        let job = WorkflowSyncJob::start(
            self.ready,
            identity,
            budget,
            limit,
            app.execution_wake.callback(),
        )
        .map_err(|_| Error::new(E_FAIL, "Cannot start workflow sync worker"))?;
        Ok(Sync {
            receipt: self.receipt,
            job,
        })
    }
}
impl Sync {
    pub(super) fn into_job(self) -> WorkflowSyncJob {
        self.job
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.job.deadline()
    }
    /// Must be called by the UI message loop; revalidation establishes the current request's admission linearization point, not a guarantee
    /// that the workspace remains unchanged at remote commit. Later edits cancel later requests, without pretending to roll back requests already sent.
    pub(in crate::desktop::application) unsafe fn service(
        &mut self,
        app: &Application,
    ) -> Option<Result<WorkflowDelivery>> {
        // Cancellation is irreversible. Keep denying late admissions and polling
        // the report without walking the workspace again.
        let current = if self.job.is_cancelled() {
            false
        } else {
            let current = self.receipt.matches(app);
            if !current {
                self.cancel();
            }
            current
        };
        if let Some(admission) = self.job.take_admission() {
            admission.resolve(current);
        }
        self.job
            .poll()
            .map(|result| result.map_err(|message| Error::new(E_FAIL, message)))
    }
    pub(in crate::desktop::application) fn cancel(&self) {
        self.job.cancel();
    }
}
impl Application {
    pub(in crate::desktop::application) unsafe fn request_workflow_sync(
        &mut self,
        candidate: Candidate,
    ) -> Result<()> {
        let record = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_ABORT, "No workflow baseline"))?;
        if record.workflow_sync.is_some() || record.workflow_delivery.is_some() {
            return Err(Error::new(
                E_ABORT,
                "Workflow sync or unconsumed delivery already exists",
            ));
        }
        let sync = candidate.start_sync(self)?;
        self.restored_session.as_mut().unwrap().workflow_sync = Some(sync);
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn process_workflow_sync(&mut self) {
        self.reap_workspace_workflow();
        self.process_workflow_cycle();
        self.pending_workflow_ui();
        let Some(mut sync) = self
            .restored_session
            .as_mut()
            .and_then(|r| r.workflow_sync.take())
        else {
            return;
        };
        let result = sync.service(self);
        let Some(record) = self.restored_session.as_mut() else {
            return;
        };
        let Some(delivery) = result else {
            record.workflow_sync = Some(sync);
            return;
        };
        record.workflow_delivery =
            Some(delivery.map(|delivery| completion::Completion::new(sync.receipt, delivery)));
        // Only a new delivery can advance the cycle here. Otherwise worker
        // notifications wake the owning pump without another UI request scan.
        self.process_workflow_cycle();
        self.pending_workflow_ui();
    }
}
