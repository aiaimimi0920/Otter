//! Single-slot workflow preparation against the saved baseline; completion retains recheckable credentials without granting network-send permission.
use super::*;
use crate::{
    desktop::{
        capability_job::Loaded,
        session_asset_root::Root,
        session_candidate::{WorkflowJob, WorkflowReady},
    },
    session_source::Source,
};
use std::{rc::Rc, sync::Arc};
#[path = "application_workflow_archive.rs"]
pub(in crate::desktop::application) mod archive;
#[path = "application_workflow_cycle.rs"]
pub(in crate::desktop::application) mod cycle;
#[path = "application_workflow_retirement.rs"]
pub(in crate::desktop::application) mod retirement;
#[path = "application_workflow_sync.rs"]
pub(in crate::desktop::application) mod sync;
#[path = "application_workflow_ui.rs"]
pub(in crate::desktop) mod ui;

pub(in crate::desktop::application) struct Receipt {
    source: Source,
    guard: guard::Guard,
    catalog: Option<Rc<Loaded>>,
}
impl Receipt {
    unsafe fn matches(&self, app: &Application) -> bool {
        app.restore_job.is_none()
            && app.save_job.is_none()
            && app.restored_session.as_ref().is_some_and(|record| {
                record.saved_path.is_some() && record.original.same_storage(&self.source)
            })
            && crate::desktop::session_binding::catalog_matches(
                &self.catalog,
                app.catalog.borrow().loaded.as_ref(),
            )
            && app
                .saved_state
                .as_ref()
                .is_some_and(|saved| self.guard.matches_saved(app, saved))
    }
}
pub(in crate::desktop::application) struct Pending {
    receipt: Receipt,
    job: WorkflowJob,
}
pub(in crate::desktop::application) struct Candidate {
    ready: WorkflowReady,
    receipt: Receipt,
}
impl Candidate {
    pub(in crate::desktop::application) unsafe fn matches(&self, app: &Application) -> bool {
        self.receipt.matches(app) && self.ready.matches_retained_source(&self.receipt.source)
    }
    pub(in crate::desktop::application) fn ready(&self) -> &WorkflowReady {
        &self.ready
    }
}
impl Application {
    pub(in crate::desktop::application) fn workflow_reap_deadline(
        &self,
    ) -> Option<std::time::Instant> {
        let record = self.restored_session.as_ref();
        self.retired_workflow
            .as_ref()
            .and_then(retirement::Jobs::deadline)
            .into_iter()
            .chain(record.and_then(|record| {
                record
                    .workflow_cycle
                    .as_ref()?
                    .deadline(record.workflow.as_ref())
            }))
            .chain(record.and_then(|record| record.workflow_sync.as_ref()?.deadline()))
            .min()
    }

    pub(in crate::desktop::application) unsafe fn request_workflow_preparation(
        &mut self,
        root: Option<Arc<Root>>,
    ) -> Result<()> {
        self.require_workflow_slot()?;
        let record = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_ABORT, "No workflow save baseline"))?;
        if record.workflow.is_some()
            || record.workflow_sync.is_some()
            || record.workflow_delivery.is_some()
        {
            return Err(Error::new(
                E_ABORT,
                "Workflow preparation is already running",
            ));
        }
        let receipt = Receipt {
            source: record.original.clone(),
            guard: guard::Guard::capture(self)?,
            catalog: self.catalog.borrow().loaded.clone(),
        };
        if !receipt.matches(self) {
            return Err(Error::new(
                E_ABORT,
                "Workflow requires unchanged committed session",
            ));
        }
        let catalog = receipt
            .catalog
            .as_ref()
            .map_or_else(Vec::new, |loaded| loaded.handshake.arts().to_vec());
        let job = WorkflowJob::start_shared(
            receipt.source.clone(),
            root,
            catalog,
            self.execution_wake.callback(),
        )
        .map_err(|e| Error::new(E_FAIL, format!("Cannot start workflow preparation: {e}")))?;
        self.restored_session.as_mut().unwrap().workflow = Some(Pending { receipt, job });
        Ok(())
    }
    pub(in crate::desktop::application) fn cancel_workflow_preparation(&self) {
        if let Some(pending) = self
            .restored_session
            .as_ref()
            .and_then(|record| record.workflow.as_ref())
        {
            pending.job.cancel();
        }
    }
    pub(in crate::desktop::application) unsafe fn take_workflow_preparation(
        &mut self,
    ) -> Result<Option<Candidate>> {
        let result = self
            .restored_session
            .as_mut()
            .and_then(|record| record.workflow.as_mut())
            .and_then(|pending| pending.job.poll());
        let Some(result) = result else {
            return Ok(None);
        };
        let pending = self
            .restored_session
            .as_mut()
            .unwrap()
            .workflow
            .take()
            .unwrap();
        let ready = result.map_err(|message| Error::new(E_ABORT, message))?;
        let candidate = Candidate {
            ready,
            receipt: pending.receipt,
        };
        if !candidate.matches(self) {
            return Err(Error::new(E_ABORT, "Workflow preparation became stale"));
        }
        Ok(Some(candidate))
    }
}

#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_workflow_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
