//! On HTTP success, publish the original Editor candidate first, then wait for exact canonical inclusion; ack and the next 32 items remain serialized.
use super::*;
impl Application {
    pub(in crate::desktop::application) unsafe fn poll_projection_edit(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let phase = std::mem::take(&mut self.projection_edit.0);
        match phase {
            Phase::Sending(mut pending) => {
                let valid = !pending.cancelled && pending.fence.matches(self);
                if !valid {
                    pending.job.request_stop();
                    pending.cancelled = true;
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 = Phase::Sending(pending);
                    return;
                };
                let result = match result {
                    Ok(Output::Edited(ready)) if valid => {
                        self.publish_projection_edit(pending.fence, ready, pending.remaining)
                    }
                    Err(error) => Err(crate::ui_text::format(
                        "app-projection-edit-unconfirmed",
                        &[("error", format!("{error:?}").into())],
                    )),
                    Ok(_) => Err(crate::ui_text::text("app-projection-edit-expired").into()),
                };
                if let Err(error) = result {
                    self.edit_error(&error);
                }
            }
            Phase::Checkpoint(saved) => {
                if !saved.fence.matches(self) {
                    self.edit_error(crate::ui_text::text(
                        "app-projection-edit-save-target-changed",
                    ));
                    return;
                }
                if !saved.durable {
                    self.projection_edit.0 = Phase::Checkpoint(saved);
                    return;
                }
                let Saved {
                    fence,
                    ready,
                    remaining,
                    ..
                } = *saved;
                let result = (|| {
                    let origin = fence.destination.origin();
                    let configuration = config(origin).map_err(|_| stale())?;
                    let lease = self
                        .authorization_owner
                        .lease(origin, now()?)
                        .map_err(|_| stale())?;
                    if !fence.current.same_grant(&lease) {
                        return Err(stale());
                    }
                    Job::start_edit_ack(configuration, lease, ready).map_err(|_| stale())
                })();
                match result {
                    Ok(job) => {
                        self.projection_edit.0 = Phase::Acknowledging(Box::new(Pending {
                            fence,
                            job,
                            cancelled: false,
                            remaining,
                        }));
                        self.edit_status(crate::ui_text::text(
                            "app-projection-edit-checkpoint-confirming",
                        ));
                    }
                    Err(error) => self.edit_error(&crate::ui_text::format(
                        "app-projection-edit-journal-retry",
                        &[("error", error.to_string().into())],
                    )),
                }
            }
            Phase::Acknowledging(mut pending) => {
                let valid = !pending.cancelled && pending.fence.matches(self);
                if !valid {
                    pending.job.request_stop();
                    pending.cancelled = true;
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 = Phase::Acknowledging(pending);
                    return;
                };
                if matches!(result, Ok(Output::EditAcknowledged)) && valid {
                    self.continue_projection_edit(pending, config);
                } else {
                    self.edit_error(crate::ui_text::text(
                        "app-projection-edit-confirm-incomplete",
                    ));
                }
            }
            Phase::Idle => {}
            Phase::Source(phase) => self.poll_projection_source(phase, config),
        }
    }
    unsafe fn publish_projection_edit(
        &mut self,
        mut fence: Fence,
        ready: Box<Ready>,
        remaining: u8,
    ) -> std::result::Result<(), String> {
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == fence.unit)
            .ok_or(crate::ui_text::text("app-projection-edit-window-closed"))?;
        let mut canvas = doc.state.canvas.borrow_mut();
        let mut slot = doc.state.projection_edit.borrow_mut();
        let owner = slot
            .as_ref()
            .ok_or(crate::ui_text::text("app-projection-edit-session-closed"))?
            .apply_ready(&ready, &mut canvas.editor)
            .map_err(|error| {
                crate::ui_text::format(
                    "app-projection-edit-unpublished",
                    &[("error", error.to_string().into())],
                )
            })?;
        let snapshot = owner.capture(&canvas.editor);
        fence.stamp = owner.stamp();
        *slot = Some(owner);
        drop(slot);
        drop(canvas);
        self.projection_edit.0 = Phase::Checkpoint(Box::new(Saved {
            fence,
            ready,
            snapshot,
            durable: false,
            remaining,
        }));
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        self.display_composition.borrow_mut().invalidate();
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
        self.edit_status(crate::ui_text::text(
            "app-projection-edit-awaiting-checkpoint",
        ));
        Ok(())
    }
    unsafe fn continue_projection_edit(
        &mut self,
        pending: Box<Pending>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        self.projection_edit.1.confirmed(&pending.fence.unit);
        let next = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == pending.fence.unit)
            .and_then(|doc| {
                let slot = doc.state.projection_edit.borrow();
                let owner = slot.as_ref()?;
                let snapshot = owner.capture(&doc.state.canvas.borrow().editor);
                let current = snapshot.editor_view().ok()?;
                let observed = crate::projection_edit::view::View::from_document(
                    snapshot.document(),
                    current.size(),
                )
                .ok()?;
                if snapshot.conflict().is_none()
                    && (snapshot.role() == crate::projection_edit::workspace::Role::Source
                        || snapshot.document().mode() == crate::projection_edit::Mode::TwoWay)
                    && !observed.changes_to(&current).ok()?.is_empty()
                {
                    Some(Selection {
                        unit: pending.fence.unit.clone(),
                        stamp: owner.stamp(),
                    })
                } else {
                    None
                }
            });
        if let Some(selection) = next {
            if pending.remaining > 1 {
                // After completion, release the original Slot/Lease/handles before admitting the next batch; at most 8 batches per explicit operation prevents endless loops under continuous editing.
                let remaining = pending.remaining - 1;
                drop(pending);
                self.start_projection_edit(selection, config, remaining, Action::Submit);
                return;
            }
            self.edit_status(crate::ui_text::text("app-projection-edit-batch-saved"));
        } else {
            let conflict = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == pending.fence.unit)
                .and_then(|doc| {
                    doc.state
                        .projection_edit
                        .borrow()
                        .as_ref()
                        .and_then(|owner| {
                            owner.capture(&doc.state.canvas.borrow().editor).conflict()
                        })
                });
            self.edit_status(if conflict.is_some() {
                crate::ui_text::text("app-projection-edit-conflict-saved")
            } else {
                crate::ui_text::text("app-projection-edit-session-checkpointed")
            });
        }
    }
}
