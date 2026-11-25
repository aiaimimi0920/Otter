//! Bounded task collection for UI owners; closure only cancels, reclamation follows completion, and exit uses explicit shutdown.
pub use super::loom_execution_work::Completion;
use super::{
    loom_execution_job::{Execution, Job},
    loom_execution_work::Work,
};
use crate::{
    graph_requests::{Request, Requests},
    workspace_id::UnitId,
};
#[path = "loom_result_commit.rs"]
mod commit;
pub use commit::CommitError;
#[cfg(test)]
#[path = "loom_execution_reap_fixture.rs"]
pub(crate) mod reap_fixture;
#[cfg(test)]
#[path = "shader_execution_reap_tests.rs"]
mod reap_tests;
#[cfg(test)]
#[path = "shader_execution_task_tests.rs"]
pub(crate) mod shader_tests;
#[derive(Debug)]
pub enum Error {
    Capacity,
    Duplicate,
    Stale,
    Allocation,
    Start(super::loom_execution_job::Error),
    ShaderStart(super::shader_execution_job::Error),
}
struct Task {
    freshness: super::loom_input_job::Freshness,
    unit: UnitId,
    request: Request,
    job: Work,
    retired: bool,
    completion: Option<Completion>,
}
#[derive(Default)]
pub struct Tasks {
    entries: Vec<Task>,
    notify: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
}
pub struct Report {
    freshness: super::loom_input_job::Freshness,
    pub unit: UnitId,
    pub request: Request,
    pub current: bool,
    pub completion: Completion,
}
pub struct Updates {
    pub request: Request,
    pub progress: Option<f64>,
    pub preview: Option<super::loom_execution_session::Update>,
}
impl Report {
    pub(crate) fn dependencies_current(
        &self,
        check: impl FnMut(&UnitId, &super::loom_input_stamp::SourceStamp) -> bool,
    ) -> bool {
        self.freshness.is_current(check)
    }
    /// Revalidate after collection and before commit; document checks cannot restore expired identities.
    pub fn is_current<'a>(
        &self,
        registry: &Requests,
        mut lookup: impl FnMut(&UnitId) -> Option<&'a crate::editor::Editor>,
    ) -> bool {
        self.current
            && registry.is_latest(&self.unit, &self.request)
            && self.freshness.is_current(|unit, stamp| {
                lookup(unit).is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
            })
    }
}
impl Tasks {
    pub fn with_notify(notify: std::sync::Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            entries: Vec::new(),
            notify: Some(notify),
        }
    }
    /// Unconsumed completion results still occupy slots, preventing unbounded results/failed leases if UI stops consuming.
    pub fn start(&mut self, execution: Execution, registry: &Requests) -> Result<(), Error> {
        let freshness = execution
            .inputs
            .freshness()
            .map_err(|_| Error::Allocation)?;
        self.start_work(
            execution.unit.clone(),
            execution.request.clone(),
            freshness,
            registry,
            |notify| {
                if execution
                    .shader
                    .as_ref()
                    .is_some_and(|runtime| runtime.is_cached())
                {
                    let mut job = super::shader_execution_job::Job::default();
                    job.start_cached(execution, notify)
                        .map_err(Error::ShaderStart)?;
                    return Ok(Work::Shader(job));
                }
                let mut job = Job::default();
                job.start_with_notify(execution, notify)
                    .map_err(Error::Start)?;
                Ok(Work::Remote(job))
            },
        )
    }
    /// Share two slots and the same UI completion transaction with remote tasks; callers bind configuration/topology and begin Results first.
    /// This entry neither approves bundle provenance, retrieves code/textures, nor changes user-side capability declarations.
    pub fn start_shader(
        &mut self,
        execution: super::shader_execution_job::Execution,
        registry: &Requests,
    ) -> Result<(), Error> {
        let freshness = execution
            .inputs
            .freshness()
            .map_err(|_| Error::Allocation)?;
        self.start_work(
            execution.unit.clone(),
            execution.request.clone(),
            freshness,
            registry,
            |notify| {
                let mut job = super::shader_execution_job::Job::default();
                job.start(execution, notify).map_err(Error::ShaderStart)?;
                Ok(Work::Shader(job))
            },
        )
    }
    fn start_work(
        &mut self,
        unit: UnitId,
        request: Request,
        freshness: super::loom_input_job::Freshness,
        registry: &Requests,
        start: impl FnOnce(super::loom_execution_signal::Notify) -> Result<Work, Error>,
    ) -> Result<(), Error> {
        if !registry.is_latest(&unit, &request) {
            return Err(Error::Stale);
        }
        if self.entries.len() >= 2 {
            return Err(Error::Capacity);
        }
        if self
            .entries
            .iter()
            .any(|task| task.unit == unit && !task.retired)
        {
            return Err(Error::Duplicate);
        }
        self.entries.try_reserve(1).map_err(|_| Error::Allocation)?;
        let job = start(
            self.notify
                .clone()
                .unwrap_or_else(|| std::sync::Arc::new(|| {})),
        )?;
        self.entries.push(Task {
            freshness,
            unit,
            request,
            job,
            retired: false,
            completion: None,
        });
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Retired tasks still own workers; slots containing only pending cleanup reports no longer schedule periodic wakes.
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.entries
            .iter()
            .filter_map(|task| task.job.deadline())
            .min()
    }
    /// Do not remove owners still cleaning up; new documents cannot inherit old requests even when reusing HWNDs.
    pub fn retain_units(&mut self, mut exists: impl FnMut(&UnitId) -> bool) {
        for task in &mut self.entries {
            if !exists(&task.unit) {
                task.retired = true;
                task.job.cancel();
            }
        }
    }
    pub fn retire_stale(&mut self, registry: &Requests) {
        for task in &mut self.entries {
            if !registry.is_latest(&task.unit, &task.request) {
                task.retired = true;
                task.job.cancel();
            }
        }
    }
    /// Main threads briefly borrow each document; dependency closure/replacement/editing or new drafts cancel old execution.
    pub(crate) fn retire_changed(
        &mut self,
        mut check: impl FnMut(&UnitId, &crate::desktop::loom_input_stamp::SourceStamp) -> bool,
    ) {
        for task in &mut self.entries {
            if !task.retired && !task.freshness.is_current(&mut check) {
                task.retired = true;
                task.job.cancel();
            }
        }
    }
    pub fn poll(&mut self) {
        for task in &mut self.entries {
            if task.completion.is_none() {
                task.completion = task.job.poll();
            }
        }
    }
    /// Collect single-slot updates after UI completion polling but before final reports, preserving the last pre-completion preview frame.
    /// Check and use must occur in the same UI scheduling segment, without nested message loops between them.
    pub fn take_updates<'a>(
        &self,
        unit: &UnitId,
        registry: &Requests,
        mut lookup: impl FnMut(&UnitId) -> Option<&'a crate::editor::Editor>,
    ) -> Option<Updates> {
        self.take_updates_checked(unit, registry, |unit, stamp| {
            lookup(unit).is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
        })
    }
    pub(crate) fn take_updates_checked(
        &self,
        unit: &UnitId,
        registry: &Requests,
        check: impl FnMut(&UnitId, &crate::desktop::loom_input_stamp::SourceStamp) -> bool,
    ) -> Option<Updates> {
        let task = self
            .entries
            .iter()
            .find(|task| &task.unit == unit && !task.retired)?;
        let (progress, preview) = task.job.take_updates();
        if !registry.is_latest(unit, &task.request)
            || !task.freshness.is_current(check)
            || (progress.is_none() && preview.is_none())
        {
            return None;
        }
        Some(Updates {
            request: task.request.clone(),
            progress,
            preview,
        })
    }
    /// current describes identity only at collection time; still check content stamps/drafts before modifying documents.
    pub fn take_completed(&mut self, registry: &Requests) -> Option<Report> {
        self.take_matching(registry, |_| true)
    }
    /// Normal application delivery must not lose owners awaiting remote cleanup; retain slots until explicit exit cleanup.
    pub fn take_settled(&mut self, registry: &Requests) -> Option<Report> {
        self.take_matching(registry, |completion| !completion.cleanup_pending())
    }
    fn take_matching(
        &mut self,
        registry: &Requests,
        ready: impl Fn(&Completion) -> bool,
    ) -> Option<Report> {
        let index = self
            .entries
            .iter()
            .position(|task| task.completion.as_ref().is_some_and(&ready))?;
        let mut task = self.entries.remove(index);
        Some(Report {
            freshness: task.freshness,
            current: !task.retired && registry.is_latest(&task.unit, &task.request),
            unit: task.unit,
            request: task.request,
            completion: task.completion.take().expect("completed task"),
        })
    }
    /// Only for cleanup after the last window exits; cancel all first, then join in sequence rather than delaying cancellation behind serial waits.
    pub fn shutdown(&mut self) -> Vec<Report> {
        for task in &mut self.entries {
            task.retired = true;
            task.job.cancel();
        }
        self.entries
            .drain(..)
            .map(|mut task| {
                let completion = task
                    .completion
                    .take()
                    .or_else(|| task.job.shutdown())
                    .expect("task has completion or worker");
                Report {
                    freshness: task.freshness,
                    unit: task.unit,
                    request: task.request,
                    current: false,
                    completion,
                }
            })
            .collect()
    }
}
