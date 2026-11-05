//! 应用级远程会话集合；容量包含停止但未回收的线程，退出后禁止再次启动。
use super::session::{Outcome, Session};
use crate::{
    byte_budget::Budget,
    device_authorization_config::Config,
    device_authorization_lease::Lease,
    live_relay_catalog_runtime::{Catalog, Error as CatalogError, Job},
    live_relay_control_runtime::Target,
    live_relay_viewer_identity::ControlEvent,
};
use std::{io, os::windows::io::BorrowedHandle, sync::Arc};
pub const MAX_SESSIONS: usize = 4;
struct Entry {
    origin: String,
    key: Arc<ViewerKey>,
    binding: Option<ViewerHost>,
    session: Session,
}
pub struct Event {
    pub origin: String,
    pub outcome: Outcome,
}
#[derive(Default)]
pub struct Sessions {
    sources: Vec<sources::SourceEntry>,
    entries: Vec<Entry>,
    catalog_job: Option<Job>,
    catalog: Option<Catalog>,
    projection_job: Option<projection::Pending>,
    // 单槽 Accept 的未结交接，不是第二队列/网络 owner；领取前禁止下次任务。
    projection_abandoned: Option<crate::projection_delivery::journal::Ticket>,
    projection_sender_abandoned: Option<crate::projection_delivery::sender::Request>,
    surface_jobs: Vec<crate::surface_lifecycle_runtime::Job>,
    surface_outcomes: Vec<crate::surface_lifecycle_runtime::Outcome>,
    resource_jobs: Vec<crate::surface_resource_runtime::Job>,
    resource_ready: Vec<crate::surface_resource_runtime::Ready>,
    event_jobs: Vec<crate::surface_event_runtime::Job>,
    event_outcomes: Vec<crate::surface_event_runtime::Outcome>,
    event_receipts: Vec<crate::desktop::surface_authorized_window::EventReceipt>,
    decision_jobs: Vec<crate::surface_decision_runtime::Job>,
    batch_jobs: Vec<(usize, crate::surface_event_batch_runtime::Job)>,
    batch_decision_jobs: Vec<(usize, crate::surface_batch_decision_runtime::Job)>,
    batch_continuation_jobs: Vec<(usize, crate::surface_event_batch_runtime::continuation::Job)>,
    batch_decision_rejected: Vec<crate::surface_batch_decision_runtime::Outcome>,
    batch_outcomes: Vec<crate::surface_event_batch_runtime::Outcome>,
    batch_receipts: Vec<crate::desktop::surface_authorized_window::BatchReceipt>,
    attach_jobs: Vec<crate::surface_attach_runtime::Job>,
    attach_outcomes: Vec<crate::surface_attach_runtime::Outcome>,
    recovery_jobs: Vec<crate::surface_recovery_runtime::Job>,
    recovery_outcomes: Vec<crate::surface_recovery_runtime::Outcome>,
    recovery_file_job: Option<recovery_file::Job>,
    recovery_file: Option<io::Result<Vec<crate::surface_recovery_record::Record>>>,
    closed: bool,
}
impl Sessions {
    /// 一次只允许一个目录请求；刷新失败也不展示上一轮目录。
    pub fn discover(&mut self, config: Config, lease: Lease) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.catalog = None;
        if self.catalog_job.is_some() || self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.catalog_job = Some(Job::start(config, lease)?);
        Ok(())
    }
    /// 服务选择切换时清空目录并取消旧请求，未回收线程仍然占槽位。
    pub fn cancel_discovery(&mut self) {
        self.catalog = None;
        if let Some(job) = &self.catalog_job {
            job.request_stop();
        }
    }
    pub fn take_catalog(&mut self) -> Option<Catalog> {
        self.catalog.take()
    }
    pub fn discovering(&self) -> bool {
        self.catalog_job.is_some()
    }
    pub fn poll_catalog(&mut self) -> Option<CatalogError> {
        let result = self.catalog_job.as_mut()?.try_join()?;
        self.catalog_job.take();
        match result {
            Ok(catalog) if !self.closed => {
                self.catalog = Some(catalog);
                None
            }
            Ok(_) | Err(CatalogError::Stopped) => None,
            Err(error) => Some(error),
        }
    }
    pub fn start(
        &mut self,
        config: Config,
        lease: Lease,
        target: Target,
        budget: Arc<Budget>,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<()> {
        self.insert_session(config.origin().into(), target.session.clone(), || {
            Session::start(config, lease, target, budget, consume)
        })
    }
    fn insert_session(
        &mut self,
        origin: String,
        target: String,
        create: impl FnOnce() -> io::Result<Session>,
    ) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.entries.try_reserve(1).map_err(io::Error::other)?;
        let key = Arc::new(ViewerKey::new(&origin, &target));
        let mut session = create()?;
        // 此时未创建窗口；只登记调用方明确发起的显示意图。
        session.show().map_err(io::Error::other)?;
        self.entries.push(Entry {
            origin,
            key,
            binding: None,
            session,
        });
        Ok(())
    }
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut index = 0;
        while index < self.entries.len() {
            if let Some(outcome) = self.entries[index].session.poll() {
                let entry = self.entries.swap_remove(index);
                events.push(Event {
                    origin: entry.origin,
                    outcome,
                });
            } else {
                index += 1;
            }
        }
        events
    }
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.entries
            .iter()
            .flat_map(|entry| entry.session.wait_handles())
            .chain(self.catalog_job.iter().filter_map(Job::wait_handle))
            .chain(self.source_handles())
            .chain(
                self.projection_job
                    .iter()
                    .filter_map(|pending| pending.handle()),
            )
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
                self.recovery_file_job
                    .iter()
                    .map(recovery_file::Job::handle),
            )
            .chain(
                self.recovery_jobs
                    .iter()
                    .filter_map(crate::surface_recovery_runtime::Job::wait_handle),
            )
            .chain(
                self.surface_jobs
                    .iter()
                    .filter_map(crate::surface_lifecycle_runtime::Job::wait_handle),
            )
            .chain(
                self.attach_jobs
                    .iter()
                    .filter_map(crate::surface_attach_runtime::Job::wait_handle),
            )
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.cancel_sources(|_, _, _| false);
        self.cancel_discovery();
        self.cancel_projection();
        self.cancel_surface_requests();
        for entry in &mut self.entries {
            entry.session.stop();
        }
    }
    pub fn finished(&self) -> bool {
        if !self.recovery_jobs.is_empty() {
            return false;
        }
        self.entries.is_empty()
            && self.sources.is_empty()
            && self.catalog_job.is_none()
            && self.projection_job.is_none()
            && self.surface_jobs.is_empty()
            && self.resource_jobs.is_empty()
            && self.event_jobs.is_empty()
            && self.decision_jobs.is_empty()
            && self.batch_jobs.is_empty()
            && self.batch_decision_jobs.is_empty()
            && self.batch_continuation_jobs.is_empty()
            && self.attach_jobs.is_empty()
            && self.recovery_file_job.is_none()
    }
    #[cfg(test)]
    pub(crate) fn insert_for_test(&mut self, origin: String, session: Session) {
        assert!(!self.closed && self.occupied() < MAX_SESSIONS);
        let key = Arc::new(ViewerKey::new(&origin, "test:inserted"));
        self.entries.push(Entry {
            origin,
            key,
            binding: None,
            session,
        });
    }
}
#[path = "composition_relay_sources.rs"]
mod sources;
pub(crate) use sources::SourceCapture;
#[path = "composition_relay_viewers.rs"]
mod viewers;
pub(crate) use viewers::ViewerHost;
pub use viewers::{ViewerKey, ViewerState};
#[path = "composition_relay_attach.rs"]
mod attach;
#[path = "composition_projection_inbox.rs"]
mod projection;
#[path = "composition_relay_recovery.rs"]
mod recovery;
#[path = "composition_relay_recovery_file.rs"]
mod recovery_file;
#[path = "composition_relay_surface_decisions.rs"]
mod surface_decisions;
pub use surface_decisions::ConfirmationAction;
#[path = "composition_relay_batch_confirmation.rs"]
mod batch_confirmation;
#[path = "composition_relay_batch_continuations.rs"]
mod batch_continuations;
#[path = "composition_relay_batch_resume.rs"]
mod batch_resume;
pub use batch_resume::BatchResumeAction;
#[path = "composition_relay_batch_decisions.rs"]
mod batch_decisions;
pub use batch_confirmation::BatchConfirmationAction;
#[path = "composition_relay_batch_input.rs"]
mod batch_input;
#[path = "composition_relay_surface_batches.rs"]
mod surface_batches;
#[path = "composition_relay_surface_events.rs"]
mod surface_events;
#[path = "composition_relay_surface_outputs.rs"]
mod surface_outputs;
#[path = "composition_relay_surface_presentations.rs"]
mod surface_presentations;
#[path = "composition_relay_surface_resources.rs"]
mod surface_resources;
#[path = "composition_relay_surfaces.rs"]
mod surfaces;
impl Drop for Sessions {
    fn drop(&mut self) {
        self.close();
    }
}
#[cfg(test)]
#[path = "composition_relay_sessions_tests.rs"]
mod tests;
