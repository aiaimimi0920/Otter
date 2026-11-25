//! Workspace publication transfers old workers without joining them on the HWND swap path.
use super::*;
use crate::desktop::session_candidate::{WorkflowArchiveJob, WorkflowSyncJob};

pub(in crate::desktop::application) struct Jobs {
    preparation: Option<WorkflowJob>,
    archive: Option<WorkflowArchiveJob>,
    sync: Option<WorkflowSyncJob>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{
        loom_execution_tasks::reap_fixture::{Gate, WAIT},
        Apartment,
    };

    #[test]
    fn retired_workflow_releases_slot_after_its_only_notification() {
        let _apartment = Apartment::new().unwrap();
        unsafe {
            let instance = crate::desktop::workspace_window_fixture::register();
            let document =
                DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
            let mut app = Application::new(instance, document);
            let pump = execution_test_pump::Pump::new();
            let wake = app.execution_wake.callback();
            let gate = Gate::new(move || wake());
            let job = WorkflowJob::start_shared(
                br#"{"documentRevision":1,"stickers":[],"links":[]}"#.to_vec().into(),
                None,
                Vec::new(),
                gate.notify,
            )
            .unwrap();
            job.cancel();
            app.retired_workflow = Some(Jobs {
                preparation: Some(job),
                archive: None,
                sync: None,
            });
            gate.noticed.recv_timeout(WAIT).unwrap();
            assert!(PeekMessageW(
                &mut MSG::default(),
                None,
                WM_APP + 92,
                WM_APP + 92,
                PM_REMOVE
            )
            .as_bool());
            app.pending_windows(false).unwrap();
            assert!(app.require_workflow_slot().is_err());
            assert!(app.workflow_reap_deadline().is_some());
            while pump.step(&mut app) != WM_NULL {}
            assert!(app.retired_workflow.is_some());
            gate.release.send(()).unwrap();
            while app.retired_workflow.is_some() {
                pump.step(&mut app);
            }
            assert!(app.require_workflow_slot().is_ok());
            assert!(app.workflow_reap_deadline().is_none());
            assert!(app.take_workflow_delivery().is_none());
            assert!(gate.noticed.try_recv().is_err());
        }
    }
}
impl Jobs {
    fn take(record: &mut restore_batch::Record) -> Option<Self> {
        let jobs = Self {
            preparation: record.workflow.take().map(|pending| pending.job),
            archive: record
                .workflow_cycle
                .take()
                .and_then(cycle::Cycle::into_archive_job),
            sync: record.workflow_sync.take().map(sync::Sync::into_job),
        };
        if let Some(job) = &jobs.preparation {
            job.cancel();
        }
        if let Some(job) = &jobs.archive {
            job.cancel();
        }
        if let Some(job) = &jobs.sync {
            job.cancel();
        }
        (!jobs.is_empty()).then_some(jobs)
    }

    fn is_empty(&self) -> bool {
        self.preparation.is_none() && self.archive.is_none() && self.sync.is_none()
    }

    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.preparation
            .as_ref()
            .and_then(WorkflowJob::deadline)
            .into_iter()
            .chain(self.archive.as_ref().and_then(WorkflowArchiveJob::deadline))
            .chain(self.sync.as_ref().and_then(WorkflowSyncJob::deadline))
            .min()
    }

    fn reap(&mut self) {
        // Poll only completed work. No old receipt is retained, no archive can
        // commit, and no old admission is answered affirmatively. An already
        // sent message can still have committed remotely; never retry it here.
        reap(&mut self.preparation, WorkflowJob::poll);
        reap(&mut self.archive, WorkflowArchiveJob::poll);
        reap(&mut self.sync, WorkflowSyncJob::poll);
    }
}

fn reap<T, R>(slot: &mut Option<T>, poll: impl FnOnce(&mut T) -> Option<R>) {
    if slot.as_mut().and_then(poll).is_some() {
        *slot = None;
    }
}

impl Application {
    pub(in crate::desktop::application) fn retire_workspace_workflow(
        &mut self,
        previous: &mut restore_batch::Record,
    ) {
        if let Some(jobs) = Jobs::take(previous) {
            // All workflow starts share the gate below, so another switch can
            // never overwrite a still-retiring generation. Job Drop joins at shutdown.
            assert!(self.retired_workflow.is_none());
            self.retired_workflow = Some(jobs);
        }
    }

    pub(super) fn reap_workspace_workflow(&mut self) {
        if let Some(jobs) = &mut self.retired_workflow {
            jobs.reap();
            if jobs.is_empty() {
                self.retired_workflow = None;
            }
        }
    }

    pub(super) fn require_workflow_slot(&self) -> Result<()> {
        if self.retired_workflow.is_some() {
            return Err(Error::new(
                E_ABORT,
                "Previous workspace workflow is still stopping",
            ));
        }
        Ok(())
    }
}
