//! 源端初始化owner复用原协作单槽；一个目标落精确Checkpoint/ack后才处理下一目标。
use super::*;
use crate::{
    desktop::projection_edit_workspace::{Base, Stamp},
    projection_delivery::sender::binding::{Binding, Candidate},
    projection_delivery_runtime::edit::source as flow,
    projection_edit::journal::Context,
    workspace_id::UnitId,
};
#[path = "application_projection_edit_source_poll.rs"]
mod poll;
#[path = "application_projection_edit_source_start.rs"]
mod start;
pub(super) enum Phase {
    Sending(Pending),
    Checkpoint(Saved),
    Acknowledging(Pending),
}
struct Guard {
    unit: UnitId,
    workspace: crate::workspace::IdentityStamp,
    context: Context,
    current: Lease,
    stamp: Option<Stamp>,
    targets: Vec<String>,
    explicit: bool,
    scheduled: Option<String>,
    target: Option<Binding>,
}
pub(super) struct Pending {
    guard: Guard,
    job: Job,
    source: Option<(super::super::projection_sender::scope::Scope, Base)>,
    cancelled: bool,
}
pub(super) struct Saved {
    guard: Guard,
    ready: Box<Ready>,
    snapshot: Snapshot,
    durable: bool,
}
impl Phase {
    pub(super) fn wait_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        match self {
            Self::Sending(p) | Self::Acknowledging(p) => p.job.wait_handle(),
            _ => None,
        }
    }
    pub(super) fn deadline(&self, now: u64) -> Option<std::time::Instant> {
        let lease = match self {
            Self::Sending(p) | Self::Acknowledging(p) if !p.cancelled => &p.guard.current,
            Self::Checkpoint(saved) => &saved.guard.current,
            _ => return None,
        };
        (!lease.is_revoked())
            .then(|| {
                std::time::Instant::now().checked_add(std::time::Duration::from_millis(
                    lease.expires_at_ms().saturating_sub(now),
                ))
            })
            .flatten()
    }
    pub(super) fn committed(
        &mut self,
        proof: Option<&crate::session_projection_scope::Proof>,
        source: &crate::session_source::Source,
        workspace: &crate::workspace::Workspace,
    ) {
        if let Self::Checkpoint(saved) = self {
            saved.durable |= saved.guard.workspace.matches(workspace)
                && proof.is_some_and(|proof| {
                    proof.is_for(source) && proof.includes_edit(&saved.snapshot)
                });
        }
    }
    pub(super) fn stop(&mut self) {
        match self {
            Self::Sending(p) | Self::Acknowledging(p) => {
                p.cancelled = true;
                p.job.request_stop();
            }
            Self::Checkpoint(saved) => saved.guard.current.revoke(),
        }
    }
}
impl Guard {
    unsafe fn matches(&self, app: &Application, candidate: Option<&Candidate>) -> bool {
        let view = app.authorization.borrow();
        !view.closed
            && view.origin.as_deref() == Some(self.context.origin())
            && !app.exit_requested.get()
            && !app.checkpoint_is_exiting()
            && app.restore_job.is_none()
            && app.projection_checkpoint_available()
            && self.workspace.matches(&app.workspace)
            && self
                .current
                .validate(self.context.origin(), now().unwrap_or(u64::MAX))
                .is_ok()
            && app
                .restored_session
                .as_ref()
                .and_then(|r| r.projection_scope)
                == Some(self.context.scope())
            && self
                .target
                .as_ref()
                .is_none_or(|expected| self.target_matches(app, expected, candidate))
            && app.documents.iter().any(|doc| {
                doc.is_alive()
                    && doc.state.unit_id == self.unit
                    && self.stamp.as_ref().is_none_or(|stamp| {
                        doc.state
                            .projection_edit
                            .borrow()
                            .as_ref()
                            .is_some_and(|owner| {
                                owner.stamp() == *stamp
                                    && owner
                                        .capture(&doc.state.canvas.borrow().editor)
                                        .suspended()
                                        .is_none()
                            })
                    })
            })
    }
    fn target_matches(
        &self,
        app: &Application,
        expected: &Binding,
        candidate: Option<&Candidate>,
    ) -> bool {
        let Some(candidate) = candidate else {
            return app
                .projection_source_targets(&self.unit, self.context.origin())
                .iter()
                .any(|target| {
                    target.binding.envelope() == expected.envelope()
                        && target.binding.target() == expected.target()
                });
        };
        // 发布candidate后旧durable scope暂时失配；只核本owner持有的原/新allocation和索引，
        // 不授予已保存事实。真正commit后仍须回到普通durable包含关系，再启动ack。
        let next = candidate.binding();
        app.restored_session
            .as_ref()
            .is_some_and(|record| record.original.same_storage(candidate.source()))
            && app.workspace_checkpoint.as_ref().is_some_and(|owner| {
                owner
                    .sender_source()
                    .is_some_and(|source| source.same_storage(candidate.expected()))
                    && owner.sender_index().is_some_and(|index| {
                        index.is_for(candidate.expected()) && index.includes(expected)
                    })
            })
            && candidate.index().is_for(candidate.source())
            && candidate.index().includes(next)
            && next.envelope() == expected.envelope()
            && next.target() == expected.target()
            && !next.stopped()
            && app.projection_sender.source_request(next).is_some()
    }
}
