//! 已保存显式路由关联的持续发送owner；单在途、64项最旧到期优先，只有真实canonical能启动。
use super::*;
use std::time::{Duration, Instant};
#[path = "application_projection_sender_update_schedule.rs"]
mod schedule;
pub(super) use schedule::Schedule;
#[path = "application_projection_sender_update_publish.rs"]
mod publish;

pub(super) struct Pending {
    scope: scope::Scope,
    expected: Binding,
    request: Request,
    pub(super) current: Lease,
    pub(super) stopped: bool,
}
impl Application {
    fn sender_update_idle(&self) -> bool {
        let view = self.authorization.borrow();
        !view.closed
            && !self.projection_edit.busy()
            && !view.busy
            && !view.projection.busy
            && !view.targets.busy
            && matches!(self.projection_sender.phase, Phase::Idle)
            && self.projection_sender.group.is_none()
            && self.projection_sender.stop_group.is_none()
            && !self.projection_sender.pending_local()
            && self.projection_sender.cleanup.network.is_none()
            && !self.remote_sessions.projection_pending()
            && !self.projection_receive.busy()
            && !self.exit_requested.get()
            && self.save_job.is_none()
            && self.restore_job.is_none()
            && self.projection_sender_scope().is_some()
            && view.origin.as_deref().is_some_and(|origin| {
                now().is_ok_and(|now| self.authorization_owner.token(origin, now).is_some())
            })
    }
    pub(super) unsafe fn process_sender_updates(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
        now: Instant,
    ) {
        self.projection_sender.updates.wake = None;
        if !self.sender_update_idle() {
            return;
        }
        let origin = self.authorization.borrow().origin.clone();
        let candidates = self
            .workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.sender_index())
            .into_iter()
            .flat_map(|index| index.entries())
            .filter_map(|binding| {
                if binding.stopped()
                    || origin.as_deref() != Some(binding.envelope().origin())
                    || !self.projection_sender_inclusion(binding)
                {
                    return None;
                }
                let request = self
                    .projection_sender
                    .profile
                    .entries()?
                    .iter()
                    .find(|request| {
                        !request.intent().cancelled()
                            && binding.matches_intent(request.intent())
                            && !request.stop_requested()
                    })?;
                Some((request.clone(), binding.clone()))
            })
            .collect();
        self.projection_sender.updates.reconcile(candidates, now);
        if let Some(index) = self.projection_sender.updates.due(now) {
            let request = self.projection_sender.updates.entries[index]
                .request
                .clone();
            let expected = self.projection_sender.updates.entries[index]
                .binding
                .clone();
            if self
                .start_sender_update(index, request.clone(), expected, config)
                .is_err()
            {
                self.projection_sender.updates.settle(&request, None, now);
                self.sender_status("投送同步暂缓，保留当前内容与关联");
            }
        }
        if matches!(self.projection_sender.phase, Phase::Idle) {
            self.projection_sender.updates.wake = self
                .projection_sender
                .updates
                .entries
                .iter()
                .map(|entry| entry.due)
                .min();
        }
    }
    unsafe fn start_sender_update(
        &mut self,
        index: usize,
        request: Request,
        expected: Binding,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        let unit = crate::workspace_id::UnitId::parse(expected.unit()).ok_or_else(scope::stale)?;
        match self.projection_source_snapshot(&unit) {
            Ok(Some(_)) => {
                let id = expected.envelope().projection_id().to_owned();
                if self.start_projection_source(unit, Some(vec![id]), false, config) {
                    return Ok(());
                }
                return Err(scope::stale());
            }
            Err(error) => return Err(error),
            Ok(None) => {}
        }
        let (scope, snapshot) = scope::Scope::capture(self, &unit)?;
        let render = !self.projection_sender.updates.entries[index]
            .stamp
            .as_ref()
            .is_some_and(|stamp| stamp.matches(self));
        let origin = expected.envelope().origin();
        let prepared = config(origin).map_err(|_| scope::stale())?;
        if self
            .projection_sender
            .profile
            .input(prepared.identity_root())
            .is_none()
        {
            return Err(scope::stale());
        }
        let current = self
            .authorization_owner
            .lease(origin, now()?)
            .map_err(|_| scope::stale())?;
        let lease = self
            .authorization_owner
            .lease(origin, now()?)
            .map_err(|_| scope::stale())?;
        lease.notify_revocation(self.execution_wake.callback());
        let source = scope.binding_source.clone();
        let render_source = source.clone();
        self.remote_sessions
            .update_projection(
                prepared,
                lease,
                crate::projection_delivery_runtime::sender_update::Request {
                    request: request.clone(),
                    expected: expected.clone(),
                    source,
                    prepare: Box::new(move |lease| {
                        if !render {
                            return Ok(None);
                        }
                        crate::desktop::export::projection::target_frame_with_input(
                            &render_source,
                            unit.as_str(),
                            &snapshot.image,
                            &snapshot.editor,
                            crate::desktop::export::projection::Presentation {
                                opacity: snapshot.opacity,
                                kind: snapshot.kind,
                                geometry: snapshot.geometry,
                            },
                            snapshot.render_input(),
                            lease.preparation_cancel(),
                        )
                        .map(Some)
                        .map_err(|_| {
                            crate::projection_delivery_runtime::Error::Sender(SenderError::Snapshot)
                        })
                    }),
                },
            )
            .map_err(|_| scope::stale())?;
        self.projection_sender.phase = Phase::Updating(Box::new(Pending {
            scope,
            expected,
            request,
            current,
            stopped: false,
        }));
        Ok(())
    }
    pub(super) unsafe fn poll_sender_update(&mut self, mut pending: Box<Pending>) {
        if pending.stopped
            || !pending.scope.matches(self)
            || !self.sender_current(&pending.current, pending.expected.envelope().origin())
        {
            pending.stopped = true;
            self.remote_sessions.cancel_projection();
        }
        if self.remote_sessions.projection_pending() {
            self.projection_sender.phase = Phase::Updating(pending);
        } else {
            self.projection_sender
                .updates
                .settle(&pending.request, None, Instant::now());
        }
    }
}
