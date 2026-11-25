//! Initialization responses atomically publish annotations/canonical candidates before ack after a real default Checkpoint; cancellation/late results preserve original stage and pending state.
use super::*;
impl Application {
    pub(in crate::desktop::application::projection_edit) unsafe fn poll_projection_source(
        &mut self,
        phase: Box<Phase>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        match *phase {
            Phase::Sending(mut pending) => {
                let valid = !pending.cancelled
                    && pending.guard.matches(self, None)
                    && pending.source.as_ref().is_some_and(|(scope, base)| {
                        scope.matches_basis(self)
                            && self
                                .documents
                                .iter()
                                .find(|doc| doc.state.unit_id == pending.guard.unit)
                                .is_some_and(|doc| base.matches(&doc.state.canvas.borrow().editor))
                    });
                if !valid {
                    pending.cancelled = true;
                    pending.job.request_stop();
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 =
                        super::super::Phase::Source(Box::new(Phase::Sending(pending)));
                    return;
                };
                let unit = pending.guard.unit.clone();
                let scheduled = pending.guard.scheduled.clone();
                let result = match result {
                    Ok(Output::Edited(ready)) if valid => {
                        self.publish_projection_source(pending, ready, config)
                    }
                    Err(error) => Err(crate::ui_text::format(
                        "app-projection-source-unconfirmed",
                        &[("error", format!("{error:?}").into())],
                    )),
                    _ => Err(crate::ui_text::text("app-projection-source-response-expired").into()),
                };
                if let Err(error) = result {
                    self.settle_projection_source(&unit, scheduled.as_deref(), false);
                    self.edit_error(&error);
                }
            }
            Phase::Checkpoint(saved) => {
                let candidate = (!saved.durable)
                    .then(|| saved.ready.source.as_ref()?.candidate.as_ref())
                    .flatten();
                if !saved.guard.matches(self, candidate) {
                    self.settle_projection_source(
                        &saved.guard.unit,
                        saved.guard.scheduled.as_deref(),
                        false,
                    );
                    self.edit_error(crate::ui_text::text("app-projection-source-save-aborted"));
                    return;
                }
                if !saved.durable {
                    self.projection_edit.0 =
                        super::super::Phase::Source(Box::new(Phase::Checkpoint(saved)));
                    return;
                }
                let result = (|| {
                    let origin = saved.guard.context.origin();
                    let prepared = config(origin).map_err(|_| stale())?;
                    let lease = self
                        .authorization_owner
                        .lease(origin, now()?)
                        .map_err(|_| stale())?;
                    if !lease.same_grant(&saved.guard.current)
                        || saved
                            .ready
                            .source
                            .as_ref()
                            .and_then(|source| source.candidate.as_ref())
                            .is_some_and(|candidate| {
                                !self.projection_sender_inclusion(candidate.binding())
                            })
                    {
                        return Err(stale());
                    }
                    Job::start_edit_ack(prepared, lease, saved.ready).map_err(|_| stale())
                })();
                match result {
                    Ok(job) => {
                        self.projection_edit.0 =
                            super::super::Phase::Source(Box::new(Phase::Acknowledging(Pending {
                                guard: saved.guard,
                                job,
                                source: None,
                                cancelled: false,
                            })));
                        self.edit_status(crate::ui_text::text(
                            "app-projection-source-checkpoint-confirming",
                        ));
                    }
                    Err(error) => {
                        self.settle_projection_source(
                            &saved.guard.unit,
                            saved.guard.scheduled.as_deref(),
                            false,
                        );
                        self.edit_error(&crate::ui_text::format(
                            "app-projection-source-confirm-retry",
                            &[("error", error.to_string().into())],
                        ));
                    }
                }
            }
            Phase::Acknowledging(mut pending) => {
                let valid = !pending.cancelled && pending.guard.matches(self, None);
                if !valid {
                    pending.cancelled = true;
                    pending.job.request_stop();
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 =
                        super::super::Phase::Source(Box::new(Phase::Acknowledging(pending)));
                    return;
                };
                if valid && matches!(result, Ok(Output::EditAcknowledged)) {
                    let guard = pending.guard;
                    drop(pending.job);
                    self.continue_projection_source(guard, config);
                } else {
                    self.settle_projection_source(
                        &pending.guard.unit,
                        pending.guard.scheduled.as_deref(),
                        false,
                    );
                    let reason = match result {
                        Err(error) => format!("{error:?}"),
                        _ => crate::ui_text::text("app-projection-response-mismatch").into(),
                    };
                    self.edit_error(&crate::ui_text::format(
                        "app-projection-source-confirm-incomplete",
                        &[("error", reason.to_string().into())],
                    ));
                }
            }
        }
    }
    unsafe fn publish_projection_source(
        &mut self,
        mut pending: Pending,
        ready: Box<Ready>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> std::result::Result<(), String> {
        let source = ready
            .source
            .as_ref()
            .ok_or(crate::ui_text::text("app-projection-source-result-missing"))?;
        let targets =
            self.projection_source_targets(&pending.guard.unit, pending.guard.context.origin());
        if !targets.iter().any(|target| target.binding == source.target) {
            return Err(crate::ui_text::text("app-projection-source-target-changed").into());
        }
        if let Some(candidate) = &source.candidate {
            if !candidate
                .expected()
                .same_storage(&pending.source.as_ref().unwrap().0.binding_source)
                || !candidate.index().is_for(candidate.source())
                || self.restored_session.is_none()
            {
                return Err(crate::ui_text::text("app-projection-source-canonical-changed").into());
            }
        }
        let route = targets
            .iter()
            .find(|target| target.binding.envelope().projection_id() == source.frozen.anchor())
            .map(|target| target.binding.edit_route());
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == pending.guard.unit)
            .ok_or(crate::ui_text::text("app-projection-source-window-closed"))?;
        let mut canvas = doc.state.canvas.borrow_mut();
        let mut slot = doc.state.projection_edit.borrow_mut();
        let unchanged = ready.ticket.is_none()
            && source.candidate.is_none()
            && slot
                .as_ref()
                .is_some_and(|owner| owner.authoritative().document() == &ready.document);
        pending
            .guard
            .targets
            .retain(|id| id != source.target.envelope().projection_id());
        pending.guard.target = Some(source.target.clone());
        if unchanged {
            drop(slot);
            drop(canvas);
            drop(ready);
            drop(pending.job);
            drop(pending.source);
            self.continue_projection_source(pending.guard, config);
            return Ok(());
        }
        let route = route
            .or_else(|| slot.as_ref().map(|owner| owner.authoritative().route()))
            .ok_or(crate::ui_text::text(
                "app-projection-source-anchor-unavailable",
            ))?;
        let owner = crate::desktop::projection_edit_workspace::source::prepare(
            slot.as_ref(),
            &source.frozen,
            source.layout,
            pending.guard.context.clone(),
            route,
            ready.document.clone(),
            &canvas.editor,
        )
        .and_then(|candidate| candidate.apply(&mut canvas.editor, None))
        .map_err(|error| {
            crate::ui_text::format(
                "app-projection-source-candidate-unpublished",
                &[("error", error.to_string().into())],
            )
        })?;
        let snapshot = owner.capture(&canvas.editor);
        pending.guard.stamp = Some(owner.stamp());
        *slot = Some(owner);
        drop(slot);
        drop(canvas);
        if let Some(candidate) = &source.candidate {
            self.restored_session
                .as_mut()
                .unwrap()
                .edit_original(candidate.source().clone());
        }
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        self.display_composition.borrow_mut().invalidate();
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
        self.projection_edit.0 = super::super::Phase::Source(Box::new(Phase::Checkpoint(Saved {
            guard: pending.guard,
            ready,
            snapshot,
            durable: false,
        })));
        self.edit_status(crate::ui_text::text(
            "app-projection-source-awaiting-checkpoint",
        ));
        Ok(())
    }
    unsafe fn continue_projection_source(
        &mut self,
        guard: Guard,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        self.settle_projection_source(
            &guard.unit,
            guard
                .target
                .as_ref()
                .map(|target| target.envelope().projection_id())
                .or(guard.scheduled.as_deref()),
            true,
        );
        if !guard.targets.is_empty() && guard.explicit {
            self.start_projection_source(guard.unit, Some(guard.targets), true, config);
        } else {
            self.edit_status(crate::ui_text::text("app-projection-source-checked"));
            self.automatic_projection_edit(guard.unit, config);
        }
    }
}
