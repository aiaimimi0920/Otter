//! Application workflow cycle: prepare again against the new baseline after archive commit; every stage shares the current Record lifetime.
use super::*;
use crate::session_document::workflow_snapshot::batch::dispatch::Report;
enum Phase {
    PreparingArchive,
    Archiving(archive::Pending),
    PreparingSync,
    Syncing,
    Finished(Outcome),
}
pub(in crate::desktop::application) struct Outcome {
    pub sync: Result<Report>,
    pub current: bool,
    pub archive: Option<crate::session_file_replace::Published>,
}
pub(in crate::desktop::application) struct Cycle {
    root: Arc<Root>,
    timestamp_ms: u64,
    phase: Phase,
    publication: Option<crate::session_file_replace::Published>,
}
impl Application {
    /// root must come from explicit admission; image paths in the session never imply write permission.
    pub(in crate::desktop::application) unsafe fn request_workflow_cycle(
        &mut self,
        root: Arc<Root>,
        timestamp_ms: u64,
    ) -> Result<()> {
        if self
            .restored_session
            .as_ref()
            .is_none_or(|r| r.workflow_cycle.is_some())
        {
            return Err(Error::new(
                E_ABORT,
                "Workflow cycle already active or awaiting consumption",
            ));
        }
        if self.catalog.borrow().loaded.is_none() {
            return Err(Error::new(
                E_ABORT,
                "Workflow cycle requires admitted capability catalog",
            ));
        }
        self.request_workflow_preparation(Some(root.clone()))?;
        self.restored_session.as_mut().unwrap().workflow_cycle = Some(Cycle {
            root,
            timestamp_ms,
            phase: Phase::PreparingArchive,
            publication: None,
        });
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn process_workflow_cycle(&mut self) {
        let Some(mut cycle) = self
            .restored_session
            .as_mut()
            .and_then(|r| r.workflow_cycle.take())
        else {
            return;
        };
        if let Err(error) = cycle.step(self) {
            self.cancel_workflow_preparation();
            if let Some(sync) = self
                .restored_session
                .as_ref()
                .and_then(|r| r.workflow_sync.as_ref())
            {
                sync.cancel();
            }
            cycle.phase = Phase::Finished(Outcome {
                sync: Err(error),
                current: false,
                archive: cycle.publication.take(),
            });
        }
        if let Some(record) = self.restored_session.as_mut() {
            record.workflow_cycle = Some(cycle);
        }
    }
    pub(in crate::desktop::application) fn take_workflow_cycle(&mut self) -> Option<Outcome> {
        let record = self.restored_session.as_mut()?;
        if !matches!(record.workflow_cycle.as_ref()?.phase, Phase::Finished(_)) {
            return None;
        }
        let cycle = record.workflow_cycle.take()?;
        let Phase::Finished(outcome) = cycle.phase else {
            unreachable!()
        };
        Some(outcome)
    }
}
impl Cycle {
    pub(super) fn deadline(&self, preparation: Option<&Pending>) -> Option<std::time::Instant> {
        match &self.phase {
            Phase::PreparingArchive | Phase::PreparingSync => preparation?.job.deadline(),
            Phase::Archiving(pending) => pending.deadline(),
            _ => None,
        }
    }

    pub(super) fn into_archive_job(
        self,
    ) -> Option<crate::desktop::session_candidate::WorkflowArchiveJob> {
        match self.phase {
            Phase::Archiving(pending) => Some(pending.into_job()),
            _ => None,
        }
    }
    unsafe fn step(&mut self, app: &mut Application) -> Result<()> {
        match &mut self.phase {
            Phase::PreparingArchive => {
                if let Some(candidate) = app.take_workflow_preparation()? {
                    self.phase = Phase::Archiving(candidate.start_archive(
                        app,
                        self.root.clone(),
                        self.timestamp_ms,
                    )?);
                }
            }
            Phase::Archiving(pending) => {
                if let Some(result) = pending.poll(app) {
                    self.publication = Some(result?.commit(app)?);
                    // The old Ready was released with the archive task; the new request must carry the revision just committed.
                    app.request_workflow_preparation(Some(self.root.clone()))?;
                    self.phase = Phase::PreparingSync;
                }
            }
            Phase::PreparingSync => {
                if let Some(candidate) = app.take_workflow_preparation()? {
                    app.request_workflow_sync(candidate)?;
                    self.phase = Phase::Syncing;
                }
            }
            Phase::Syncing => {
                if let Some(result) = app.take_workflow_delivery() {
                    let completion = result?;
                    let current = completion.is_current(app);
                    self.phase = Phase::Finished(Outcome {
                        sync: Ok(completion.into_report()),
                        current,
                        archive: self.publication.take(),
                    });
                }
            }
            Phase::Finished(_) => (),
        }
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_workflow_real_loom_tests.rs"]
mod real_loom_tests;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_workflow_cycle_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_workflow_cycle_tests.rs"]
mod tests;
