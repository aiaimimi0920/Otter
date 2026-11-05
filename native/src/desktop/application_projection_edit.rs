//! 已有协作会话的提交owner；一个原网络Job/写前意图，精确默认Checkpoint后才允许后台ack。
use super::*;
use crate::{
    desktop::device_authorization_ui::{editing::Selection, Request},
    device_authorization_config::Config,
    device_authorization_lease::Lease,
    projection_delivery::journal::Ticket,
    projection_delivery_runtime::{
        edit::{Action, Input, Ready},
        Job, Output,
    },
    projection_edit::workspace::Snapshot,
};
use std::sync::Arc;
#[path = "application_projection_edit_automatic.rs"]
mod automatic;
#[path = "application_projection_edit_poll.rs"]
mod poll;
#[path = "application_projection_edit_source.rs"]
mod source;
#[path = "application_projection_edit_start.rs"]
mod start;

#[derive(Default)]
pub(super) struct Owner(Phase, automatic::Blocked);
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Sending(Box<Pending>),
    Checkpoint(Box<Saved>),
    Acknowledging(Box<Pending>),
    Source(Box<source::Phase>),
}
struct Fence {
    unit: crate::workspace_id::UnitId,
    workspace: crate::workspace::IdentityStamp,
    stamp: crate::desktop::projection_edit_workspace::Stamp,
    destination: Destination,
    current: Lease,
}
enum Destination {
    Receiver(Arc<Ticket>),
    Source(Box<crate::projection_delivery::sender::binding::Binding>),
}
impl Destination {
    fn origin(&self) -> &str {
        match self {
            Self::Receiver(ticket) => ticket.origin(),
            Self::Source(binding) => binding.envelope().origin(),
        }
    }
    fn matches(&self, app: &Application, unit: &crate::workspace_id::UnitId) -> bool {
        match self {
            Self::Receiver(ticket) => {
                app.projection_profile.recorded(ticket)
                    && app
                        .projection_sync_index()
                        .and_then(|index| index.for_ticket(ticket))
                        .is_some_and(|binding| !binding.stopped())
            }
            Self::Source(binding) => app
                .projection_source_targets(unit, self.origin())
                .iter()
                .any(|target| target.binding == **binding),
        }
    }
}
struct Pending {
    fence: Fence,
    job: Job,
    cancelled: bool,
    remaining: u8,
}
struct Saved {
    fence: Fence,
    ready: Box<Ready>,
    snapshot: Snapshot,
    durable: bool,
    remaining: u8,
}
impl Owner {
    pub(super) fn busy(&self) -> bool {
        !matches!(self.0, Phase::Idle)
    }
    pub(super) fn wait_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        match &self.0 {
            Phase::Sending(pending) | Phase::Acknowledging(pending) => pending.job.wait_handle(),
            Phase::Source(phase) => phase.wait_handle(),
            _ => None,
        }
    }
    pub(super) fn deadline(&self, now: u64) -> Option<std::time::Instant> {
        if let Phase::Source(phase) = &self.0 {
            return phase.deadline(now);
        }
        let lease = match &self.0 {
            Phase::Sending(pending) | Phase::Acknowledging(pending) if !pending.cancelled => {
                &pending.fence.current
            }
            Phase::Checkpoint(saved) => &saved.fence.current,
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
        if let Phase::Source(phase) = &mut self.0 {
            phase.committed(proof, source, workspace);
            return;
        }
        let Phase::Checkpoint(saved) = &mut self.0 else {
            return;
        };
        saved.durable |= saved.fence.workspace.matches(workspace)
            && proof
                .is_some_and(|proof| proof.is_for(source) && proof.includes_edit(&saved.snapshot));
    }
    fn stop(&mut self) {
        match &mut self.0 {
            Phase::Sending(pending) | Phase::Acknowledging(pending) => {
                pending.cancelled = true;
                pending.job.request_stop();
            }
            Phase::Checkpoint(_) => self.0 = Phase::Idle,
            Phase::Source(phase) => phase.stop(),
            _ => {}
        }
    }
    pub(super) fn shutdown(&mut self) {
        self.stop();
        self.0 = Phase::Idle; // 终态Drop回收原Job；未知网络结果保留磁盘pending，不自动重发。
    }
}
impl Fence {
    unsafe fn matches(&self, app: &Application) -> bool {
        let view = app.authorization.borrow();
        !view.closed
            && !app.exit_requested.get()
            && !app.checkpoint_is_exiting()
            && app.restore_job.is_none()
            && app.projection_checkpoint_available()
            && self.workspace.matches(&app.workspace)
            && view.origin.as_deref() == Some(self.destination.origin())
            && self
                .current
                .validate(self.destination.origin(), now().unwrap_or(u64::MAX))
                .is_ok()
            && self.destination.matches(app, &self.unit)
            && app.documents.iter().any(|doc| {
                doc.is_alive()
                    && doc.state.unit_id == self.unit
                    && doc
                        .state
                        .projection_edit
                        .borrow()
                        .as_ref()
                        .is_some_and(|owner| {
                            let snapshot = owner.capture(&doc.state.canvas.borrow().editor);
                            owner.stamp() == self.stamp
                                && snapshot.suspended().is_none()
                                && app
                                    .restored_session
                                    .as_ref()
                                    .and_then(|record| record.projection_scope)
                                    == Some(snapshot.context().scope())
                        })
            })
    }
}
fn now() -> Result<u64> {
    crate::desktop::surface_windows::now_ms()
}
fn stale() -> Error {
    Error::new(
        E_ABORT,
        "Collaboration submission destination changed or unavailable",
    )
}
