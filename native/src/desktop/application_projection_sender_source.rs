//! sender与源端协作的最小公共接缝；canonical/原意图仍为目标权威，不从目录猜既有关联。
use super::*;
use crate::projection_delivery_runtime::edit::source::Target;
impl Owner {
    pub(in crate::desktop::application) fn source_request(
        &self,
        binding: &Binding,
    ) -> Option<&Request> {
        self.profile.entries()?.iter().find(|request| {
            !request.stop_requested()
                && !request.intent().cancelled()
                && binding.matches_intent(request.intent())
        })
    }
}
impl Application {
    pub(in crate::desktop::application) fn projection_source_snapshot(
        &self,
        unit: &crate::workspace_id::UnitId,
    ) -> Result<Option<crate::projection_edit::workspace::Snapshot>> {
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == *unit)
            .ok_or_else(scope::stale)?;
        let slot = doc.state.projection_edit.borrow();
        let Some(owner) = slot.as_ref() else {
            return Ok(None);
        };
        let snapshot = owner.capture(&doc.state.canvas.borrow().editor);
        if snapshot.role() != crate::projection_edit::workspace::Role::Source {
            return Ok(None);
        }
        if snapshot.suspended().is_some() {
            return Err(scope::stale());
        }
        Ok(Some(snapshot))
    }
    pub(in crate::desktop::application) fn projection_source_targets(
        &self,
        unit: &crate::workspace_id::UnitId,
        origin: &str,
    ) -> Vec<Target> {
        self.workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.sender_index())
            .into_iter()
            .flat_map(|index| index.entries())
            .filter(|binding| {
                binding.unit() == unit.as_str()
                    && binding.envelope().origin() == origin
                    && !binding.stopped()
                    && self.projection_sender_inclusion(binding)
            })
            .filter_map(|binding| {
                let request = self.projection_sender.source_request(binding)?;
                Some(Target {
                    request: request.clone(),
                    binding: binding.clone(),
                })
            })
            .take(8)
            .collect()
    }
    pub(in crate::desktop::application) fn sender_idle_for_edit(&self) -> bool {
        !self.projection_sender.busy()
            && !self.projection_sender.pending_local()
            && self.projection_sender.cleanup.network.is_none()
            && !self.remote_sessions.projection_pending()
    }
    pub(in crate::desktop::application) unsafe fn projection_source_cached(
        &self,
        unit: &crate::workspace_id::UnitId,
        projection: &str,
    ) -> bool {
        self.projection_sender
            .updates
            .source_cached(unit, projection, |stamp| stamp.matches(self))
    }
    pub(in crate::desktop::application) unsafe fn settle_projection_source(
        &mut self,
        unit: &crate::workspace_id::UnitId,
        projection: Option<&str>,
        success: bool,
    ) {
        // 只推进本次目标；重置整Unit会使最先到期的目标永久饿死同组其他成员。
        // 精确Checkpoint后重捕弱身份，不延长PNG/canonical或任何像素租约。
        let stamp = if success {
            scope::Scope::capture(self, unit).ok().map(|v| v.0.into())
        } else {
            None
        };
        self.projection_sender.updates.source_settled(
            unit,
            projection,
            stamp,
            success,
            std::time::Instant::now(),
        );
    }
}
impl scope::Snapshot {
    pub(in crate::desktop::application) fn frozen_send_frame(
        &self,
        source: &crate::session_source::Source,
        unit: &str,
        profile: &std::path::Path,
        state: &crate::projection_edit::workspace::Snapshot,
        lease: &Lease,
    ) -> std::result::Result<
        crate::projection_delivery::sender::Frame,
        crate::projection_delivery_runtime::Error,
    > {
        use crate::projection_delivery_runtime::Error as JobError;
        let fail = || JobError::Sender(SenderError::Changed);
        if state.context().device() != lease.device_id() || state.context().unit() != unit {
            return Err(fail());
        }
        let store =
            crate::projection_edit::journal::Store::open(profile, lease.preparation_cancel())
                .map_err(|error| JobError::Edit(crate::projection_edit::Failure::Journal(error)))?;
        let frozen = store
            .source(state.context(), lease.preparation_cancel())
            .map_err(|error| JobError::Edit(crate::projection_edit::Failure::Journal(error)))?
            .ok_or_else(fail)?;
        if state.document().session_id() != frozen.session()
            || state.document().basis() != &frozen.basis()
        {
            return Err(fail());
        }
        let rendered = self.freeze(source, unit, frozen.anchor(), frozen.session(), lease)?;
        if !frozen.same_basis(&rendered) {
            return Err(fail());
        }
        frozen
            .frame(self.kind, lease.preparation_cancel())
            .map_err(JobError::Sender)
    }
    pub(in crate::desktop::application) fn freeze(
        &self,
        source: &crate::session_source::Source,
        unit: &str,
        anchor: &str,
        session: &str,
        lease: &Lease,
    ) -> std::result::Result<
        crate::projection_edit::source::Frozen,
        crate::projection_delivery_runtime::Error,
    > {
        use crate::projection_delivery_runtime::Error as JobError;
        let (frame, layout) = crate::desktop::export::projection::basis_frame(
            source,
            unit,
            &self.image,
            &self.editor,
            crate::desktop::export::projection::Presentation {
                opacity: 1.0,
                kind: self.kind,
                geometry: self.geometry,
            },
            self.render_input(),
            lease.preparation_cancel(),
        )
        .map_err(|_| JobError::Sender(SenderError::Snapshot))?;
        crate::projection_edit::source::Frozen::new(
            anchor,
            session,
            frame,
            layout,
            |basis, layout| {
                crate::desktop::projection_edit_workspace::source::initial(
                    basis,
                    layout,
                    &self.editor,
                )
            },
            lease.preparation_cancel(),
        )
        .map_err(|error| JobError::Edit(crate::projection_edit::Failure::Protocol(error)))
    }
}
