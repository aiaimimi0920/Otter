//! 生产单目标发送 owner：固定来源→原 Job 当前帧→绑定发布→精确 checkpoint；取消保原 Request。
use super::*;
use crate::{
    device_authorization_lease::Lease,
    projection_delivery::sender::{
        binding::{checkpoint::Index, Binding},
        Error as SenderError, Request, Store,
    },
    projection_delivery_runtime::{Output, SenderReady},
};
#[path = "application_projection_sender_cleanup.rs"]
mod cleanup;
#[path = "application_projection_sender_continuous.rs"]
mod continuous;
#[path = "application_projection_sender_group.rs"]
mod group;
#[path = "application_projection_sender_lifecycle.rs"]
mod lifecycle;
#[path = "application_projection_sender_poll.rs"]
mod poll;
#[path = "application_projection_sender_profile.rs"]
mod profile;
#[path = "application_projection_sender_retry.rs"]
mod retry;
#[path = "application_projection_sender_scope.rs"]
pub(super) mod scope;
#[path = "application_projection_sender_source.rs"]
mod source;
#[path = "application_projection_sender_start.rs"]
mod start;
#[path = "application_projection_sender_stop.rs"]
mod stop;
#[path = "application_projection_sender_stop_group.rs"]
mod stop_group;
#[cfg(test)]
#[path = "application_projection_sender_tests.rs"]
mod tests;
#[derive(Default)]
pub(super) struct Owner {
    profile: profile::Profile,
    phase: Phase,
    cleanup: cleanup::Owner,
    stop: stop::Owner,
    updates: continuous::Schedule,
    group: Option<group::Batch>,
    stop_group: Option<stop_group::Batch>,
}
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Waiting(Box<Pending>),
    Retry(Box<Pending>),
    Checkpoint(Box<Saved>),
    Updating(Box<continuous::Pending>),
}
struct Pending {
    origin: String,
    scope: scope::Scope,
    current: Lease,
    request: Option<Request>,
    stopped: bool,
    name: String,
}
struct Saved {
    ready: SenderReady,
    current: Lease,
    expected: Binding,
    source: crate::session_source::Source,
    previous: crate::session_source::Source,
    workspace: crate::workspace::IdentityStamp,
    retained_disk: bool,
    updated: bool,
}
impl Owner {
    pub(super) fn stop_committed(
        &mut self,
        index: Option<&Index>,
        canonical: &crate::session_source::Source,
        original: &crate::session_source::Source,
        workspace: &crate::workspace::Workspace,
    ) {
        if let Some(batch) = &mut self.stop_group {
            batch.committed(index, canonical, original, workspace);
        }
    }
    pub(super) fn busy(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
            || self.stop.busy
            || self.group.is_some()
            || self.stop_group.is_some()
    }
    pub(super) fn pending_local(&self) -> bool {
        self.cleanup.pending_local() || self.stop.pending()
    }
    pub(super) fn start(
        &mut self,
        root: std::path::PathBuf,
        notify: crate::desktop::loom_execution_signal::Notify,
    ) {
        self.profile.start(root, notify);
    }
    pub(super) fn committed(
        &mut self,
        index: Option<&Index>,
        workspace: &crate::workspace::Workspace,
    ) {
        if let Phase::Checkpoint(saved) = &mut self.phase {
            if saved.workspace.matches(workspace)
                && index.is_none_or(|index| index.includes(&saved.expected))
            {
                // 实际磁盘提交；坏/未知 index 不能当“不包含”来自动解绑可能已保存的关联。
                saved.retained_disk = true;
            }
        }
    }
    pub(super) fn deadline(&self, now: u64) -> Option<std::time::Instant> {
        let expiry = match &self.phase {
            Phase::Waiting(pending) if !pending.stopped => Some(pending.current.expires_at_ms()),
            Phase::Retry(pending) => Some(pending.current.expires_at_ms()),
            Phase::Checkpoint(saved) if !saved.retained_disk => Some(saved.current.expires_at_ms()),
            Phase::Updating(pending) if !pending.stopped => Some(pending.current.expires_at_ms()),
            _ => None,
        }
        .and_then(|expires| {
            std::time::Instant::now().checked_add(std::time::Duration::from_millis(
                expires.saturating_sub(now),
            ))
        });
        expiry
            .into_iter()
            .chain(self.cleanup.deadline())
            .chain(self.stop.deadline())
            .chain(self.updates.wake)
            .min()
    }
}
fn now() -> Result<u64> {
    crate::desktop::surface_windows::now_ms()
}
impl Application {
    fn sender_status(&self, status: &str) {
        let mut view = self.authorization.borrow_mut();
        view.sender.busy = self.projection_sender.busy();
        view.sender.status = status.into();
    }
    pub(super) fn collect_projection_sender(&mut self) {
        if let Some(request) = self.remote_sessions.take_projection_sender() {
            self.projection_sender.profile.remember(request.clone());
            match &mut self.projection_sender.phase {
                Phase::Waiting(pending) => pending.request = Some(request),
                _ if self.projection_sender.cleanup.network.is_some() => {}
                _ => self.projection_sender.cleanup.enqueue(request, false),
            }
        }
    }
    fn sender_current(&self, lease: &Lease, origin: &str) -> bool {
        let view = self.authorization.borrow();
        !view.closed
            && view.origin.as_deref() == Some(origin)
            && !self.exit_requested.get()
            && now().is_ok_and(|now| lease.validate(origin, now).is_ok())
    }
}
