//! 应用工作流周期：归档提交之后按新基线重准备；所有阶段共用当前 Record 生命周期。
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
    /// root 必须来自显式准入；不会从会话中的图片路径推导写权限。
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
                    // 旧 Ready 已随归档任务释放；新请求必须携带刚提交的 revision。
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

#[path = "application_workflow_cycle_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
#[cfg(test)]
#[path = "application_workflow_cycle_tests.rs"]
mod tests;
