//! Completion values first validate the original workspace/window/authorization; failed saves do not show receiver windows, and stops persist only after remote confirmation.
use super::*;
impl Application {
    unsafe fn account_pending_current(&self, pending: &Pending) -> bool {
        self.account_idle()
            && pending
                .participation
                .as_ref()
                .is_none_or(|stamp| self.authorization.borrow().consent.participates(stamp))
            && pending.workspace.matches(&self.workspace)
            && pending
                .cancel
                .as_ref()
                .is_none_or(|flag| !flag.load(Ordering::Acquire))
            && self.authorization.borrow().origin.as_deref() == Some(&pending.origin)
            && pending.job.valid(&pending.origin)
            && self
                .restored_session
                .as_ref()
                .is_some_and(|r| r.original.same_storage(&pending.source))
            && match &pending.guard {
                Guard::Source(scope) => scope.matches(self),
                Guard::Receive(destination) => destination.matches(self),
                Guard::Existing { unit, document } => self
                    .documents
                    .iter()
                    .find(|d| d.state.unit_id == *unit)
                    .is_some_and(|d| document.matches(d)),
            }
    }
    pub(super) unsafe fn poll_account_projection(&mut self) {
        if let Some(saved) = self.account_projection.saved.take() {
            let current = saved.workspace.matches(&self.workspace)
                && self
                    .restored_session
                    .as_ref()
                    .is_some_and(|r| r.original.same_storage(&saved.source));
            let included = current
                && self
                    .workspace_checkpoint
                    .as_ref()
                    .is_some_and(|o| o.includes_account(&saved.binding, &self.workspace));
            if included {
                if saved.binding.stopped() {
                    self.account_projection.recovery = None;
                }
                if let Some(receipt) = &saved.receive {
                    match receipt.present(self, saved.cancel.as_deref()) {
                        Ok(true) => {
                            self.authorization.borrow_mut().account.received =
                                Some(saved.binding.envelope().clone());
                        }
                        Ok(false) => {}
                        Err(error) => {
                            self.account_status(&crate::ui_text::format(
                                "app-account-display-failed",
                                &[("error", error.to_string().into())],
                            ));
                            return;
                        }
                    }
                }
                self.account_status(if saved.binding.stopped() {
                    crate::ui_text::text("app-account-unlinked")
                } else {
                    crate::ui_text::text("app-account-saved")
                });
                self.account_projection.due = Some(Instant::now() + Duration::from_secs(1));
            } else if current
                && self.projection_checkpoint_available()
                && !self.exit_requested.get()
            {
                self.account_projection.saved = Some(saved);
            } else {
                self.account_status(crate::ui_text::text("app-account-save-incomplete"));
            }
        }
        let Some(mut pending) = self.account_projection.pending.take() else {
            return;
        };
        let valid = self.account_pending_current(&pending);
        if !valid {
            pending.job.stop();
        }
        let Some(result) = pending.job.poll() else {
            self.account_projection.pending = Some(pending);
            return;
        };
        let result = if valid {
            result.and_then(|output| self.publish_account_output(&pending, output))
        } else {
            Err(stale())
        };
        if let Err(error) = result {
            self.account_status(&crate::ui_text::format(
                "app-account-sync-deferred",
                &[("error", error.to_string().into())],
            ));
            self.account_projection.due = Some(Instant::now() + Duration::from_secs(5));
        } else if self.account_projection.saved.is_none() {
            self.account_status(crate::ui_text::text("app-account-projection-checked"));
            self.account_projection.due = Some(Instant::now() + Duration::from_secs(1));
        }
    }
    unsafe fn publish_account_output(&mut self, pending: &Pending, output: Output) -> Result<()> {
        let (binding, receive) = match output {
            Output::Recovery(intents) => {
                self.account_projection.recovery = self
                    .restored_session
                    .as_ref()
                    .and_then(|r| r.projection_scope)
                    .map(|scope| (scope, pending.origin.clone()));
                self.account_projection.recovery_queue = intents.into();
                return Ok(());
            }
            Output::Unchanged => return Ok(()),
            Output::Receive(prepared) => {
                let Guard::Receive(destination) = &pending.guard else {
                    return Err(stale());
                };
                let receipt = self.publish_account_projection(destination, *prepared)?;
                (receipt.binding.clone(), Some(receipt))
            }
            Output::Patch {
                source,
                binding,
                image,
            } => {
                if let Some(image) = image {
                    let doc = self
                        .documents
                        .iter()
                        .find(|d| d.state.unit_id.as_str() == binding.unit())
                        .ok_or_else(stale)?;
                    let mut canvas = doc.state.canvas.borrow_mut();
                    if !canvas.editor.replace_source(image.base.clone()) {
                        return Err(stale());
                    }
                    canvas.sync_raster_source();
                    canvas.minified_source =
                        Some(crate::desktop::canvas::minified_source::Source::new(
                            image.original,
                            &canvas.editor,
                        ));
                    canvas.discard_device();
                    *doc.state.execution_results.borrow_mut() =
                        crate::desktop::loom_result_state::Results::new(doc.state.unit_id.clone());
                    *doc.state.resolved_display.borrow_mut() = None;
                    doc.state
                        .hit_region
                        .borrow_mut()
                        .replace_source(image.base, image.mask);
                    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                        Some(doc.window.0),
                        None,
                        false,
                    );
                }
                self.restored_session
                    .as_mut()
                    .ok_or_else(stale)?
                    .edit_original(source);
                self.saved_state = None;
                self.display_cache.borrow_mut().take();
                self.display_composition.borrow_mut().invalidate();
                (*binding, None)
            }
        };
        self.account_projection.saved = Some(Saved {
            binding,
            workspace: self.workspace.identity_stamp(),
            source: self.restored_session.as_ref().unwrap().original.clone(),
            receive,
            cancel: pending.cancel.clone(),
        });
        self.account_status(crate::ui_text::text("app-account-saving"));
        Ok(())
    }
}
