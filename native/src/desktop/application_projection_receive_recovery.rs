//! 显式已保存回执恢复：当前 canonical/显示凭证→只读核验原图→原呈现路径，不追加 Unit。
use super::*;
use crate::{
    device_authorization_config::Config,
    projection_delivery::{binding::Binding, journal::Ticket},
    projection_delivery_runtime::{Job, Output},
};
use std::sync::Arc;

pub(super) struct Pending {
    source: Source,
    workspace: crate::workspace::IdentityStamp,
    ticket: Arc<Ticket>,
    expected: Binding,
    visual: present::Visual,
    current: Lease,
    pub(super) job: Job,
}
impl Application {
    pub(super) unsafe fn projection_recovery_current(
        &self,
        ticket: &Ticket,
        expected: &Binding,
    ) -> bool {
        self.save_job.is_none()
            && self.restore_job.is_none()
            && restore_batch::guard::is_idle(self)
            && !ticket.cleanup_pending()
            && self.projection_profile.recorded(ticket)
            && self.projection_checkpoint_inclusion(ticket) == Some(expected)
            && self
                .workspace_checkpoint
                .as_ref()
                .is_some_and(|owner| owner.projection_model_current(self, ticket.unit()))
    }
    pub(in crate::desktop::application) unsafe fn start_projection_display_recovery(
        &mut self,
        ticket: Arc<Ticket>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let result = (|| {
            let view = self.authorization.borrow();
            if view.closed
                || view.busy
                || view.projection.busy
                || self.projection_receive.busy()
                || self.projection_edit.busy()
                || self.remote_sessions.projection_pending()
                || !self.projection_profile.contains(&ticket)
                || !view
                    .projection
                    .recovery
                    .saved
                    .iter()
                    .any(|old| Arc::ptr_eq(old, &ticket))
                || view.origin.as_deref() != Some(ticket.origin())
            {
                return Err(stale());
            }
            drop(view);
            let expected = self
                .projection_checkpoint_inclusion(&ticket)
                .ok_or_else(stale)?
                .clone();
            if !self.projection_recovery_current(&ticket, &expected) {
                return Err(stale());
            }
            let doc = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == ticket.unit())
                .ok_or_else(stale)?;
            let visual = present::Visual::capture(doc)?;
            let image = workspace_checkpoint::projection_proof::source(&doc.state.canvas.borrow())
                .ok_or_else(stale)?
                .clone();
            let origin = ticket.origin();
            let config = config(origin).map_err(|_| stale())?;
            let current = self.receive_lease(origin)?;
            let lease = self.receive_lease(origin)?;
            lease.notify_revocation(self.execution_wake.callback());
            let job = Job::start_recovery(config, lease, ticket.retry(), expected.clone(), image)
                .map_err(|e| {
                Error::new(E_FAIL, format!("Cannot start projection recovery: {e}"))
            })?;
            self.projection_receive.0 = Phase::Recovering(Box::new(Pending {
                source: self.restored_session.as_ref().unwrap().original.clone(),
                workspace: self.workspace.identity_stamp(),
                ticket,
                expected,
                visual,
                current,
                job,
            }));
            Ok("正在核验已保存投送…")
        })();
        self.receive_status(result);
    }
    pub(super) unsafe fn poll_projection_recovery(
        &mut self,
        mut pending: Box<Pending>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<&'static str> {
        let valid = pending.workspace.matches(&self.workspace)
            && self.receive_source_current(&pending.source, pending.ticket.unit())
            && self.projection_recovery_current(&pending.ticket, &pending.expected)
            && self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == pending.ticket.unit())
                .is_some_and(|doc| pending.visual.matches(doc));
        if !valid {
            pending.job.request_stop();
        }
        let Some(result) = pending.job.try_join() else {
            self.projection_receive.0 = Phase::Recovering(pending);
            return Ok(if valid {
                "正在核验已保存投送…"
            } else {
                "正在取消投送恢复…"
            });
        };
        // 只记录封闭错误类别和围栏类别，不格式化 Ready、凭据或远端正文。
        let mismatch = || {
            self.documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == pending.ticket.unit())
                .map_or(Some("missing-window"), |doc| pending.visual.mismatch(doc))
        };
        let ready = match result {
            Ok(Output::Received(ready)) => ready,
            Err(error) => {
                return Err(Error::new(E_ABORT, format!(
                    "Projection recovery worker failed: {error:?}; destination_current={valid}; visual={:?}",
                    mismatch()
                )));
            }
            _ => {
                return Err(Error::new(
                    E_ABORT,
                    "Projection recovery returned an unexpected output",
                ))
            }
        };
        if !valid {
            return Err(Error::new(
                E_ABORT,
                format!(
                    "Projection recovery destination changed before presentation: {:?}",
                    mismatch()
                ),
            ));
        }
        let (_, held) = ready
            .transfer(&pending.current, job::now()?)
            .map_err(|error| {
                Error::new(
                    E_ABORT,
                    format!("Projection recovery transfer failed: {error:?}"),
                )
            })?;
        self.display_projection_saved(
            Box::new(Saved {
                source: pending.source,
                held,
                expected: pending.expected,
                durable: true,
                current: pending.current,
                visual: pending.visual,
                workspace: pending.workspace,
                recovery: true,
            }),
            config,
        )
    }
}
