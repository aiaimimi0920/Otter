//! Explicit workspace autosave; Live takes a point-in-time snapshot only on model changes, shares one slot, and triggers no external workflows.
use super::*;
use std::{cell::Cell, sync::atomic::AtomicBool};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_workspace_autosave_smoke.rs"]
pub(super) mod smoke;

#[derive(Default)]
pub(in crate::desktop) struct Control {
    enabled: Cell<bool>,
    generation: Cell<u64>,
}
impl Control {
    pub(in crate::desktop) fn enabled(&self) -> bool {
        self.enabled.get()
    }
    pub(in crate::desktop) fn toggle(&self) -> Result<()> {
        let Some(next) = self.generation.get().checked_add(1) else {
            self.disable(); // Even if previously enabled, permission must not survive generation exhaustion.
            return Err(Error::new(
                E_ABORT,
                "Workspace autosave generation exhausted",
            ));
        };
        self.generation.set(next);
        self.enabled.set(!self.enabled());
        Ok(())
    }
    pub(super) fn disable(&self) {
        self.enabled.set(false);
        self.generation.set(self.generation.get().saturating_add(1));
    }
}

impl Application {
    pub(super) unsafe fn pending_workspace_save_with(
        &mut self,
        mut notify: impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if self.modals.blocked() {
            return;
        }
        self.pending_checkpoint_with(&mut notify);
        if self.checkpoint_pending() || self.checkpoint_exit_waiting() {
            return;
        }
        let result = self.poll_workspace_autosave();
        self.workspace_autosave_result(result, &mut notify);
        if self.workspace_autosave_pending.is_some() {
            return; // Explicit save/save-as intent stays in its original slot for the existing UI path to consume after the automatic task is reaped.
        }
        let mut manual_warning = false;
        self.pending_save_ui_with(|app, origin, message, warning| {
            manual_warning |= warning;
            notify(app, origin, message, warning);
        });
        if manual_warning {
            // The background path must not automatically retry a failed or canceled manual-save destination.
            self.workspace_autosave.disable();
            return;
        }
        let result = self.start_workspace_autosave();
        self.workspace_autosave_result(result, &mut notify);
        self.start_checkpoint_with(&mut notify);
    }
    unsafe fn workspace_autosave_result(
        &mut self,
        result: Result<()>,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Err(error) = result {
            self.workspace_autosave.disable();
            self.exit_requested.set(false);
            if let Some(doc) = self.documents.iter().find(|doc| doc.is_alive()) {
                notify(
                    self,
                    &doc.state.unit_id,
                    &crate::ui_text::format(
                        "app-workspace-autosave-stopped",
                        &[("error", error.to_string().into())],
                    ),
                    true,
                );
            }
        }
    }
    unsafe fn start_workspace_autosave(&mut self) -> Result<()> {
        if !self.workspace_autosave.enabled()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_after_save
            || self.open_after_save.is_some()
            || self.exit_requested.get()
            || self
                .documents
                .iter()
                .any(|doc| doc.state.open_session_requested.get())
        {
            return Ok(());
        }
        if self.save_requires_destination(false) {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-workspace-autosave-prerequisite"),
            ));
        }
        if !save_snapshot::automatic_static_ready(self)
            || self
                .saved_state
                .as_ref()
                .is_some_and(|stamp| stamp.matches_automatic(self))
        {
            return Ok(());
        }
        if !restore_batch::guard::is_idle(self) {
            return Ok(()); // Wait for the normal gesture/background-task completion message without dropping drafts or adding timers.
        }
        if self.request_automatic_save_existing()? {
            self.workspace_autosave_pending = Some(self.workspace_autosave.generation.get());
        }
        Ok(())
    }
    unsafe fn poll_workspace_autosave(&mut self) -> Result<()> {
        let Some(generation) = self.workspace_autosave_pending else {
            return Ok(());
        };
        let current = self.workspace_autosave.enabled()
            && self.workspace_autosave.generation.get() == generation
            && !self.exit_requested.get();
        if !current {
            self.cancel_save_candidate();
        }
        let result = self.poll_save_candidate();
        if matches!(result, Ok(save_control::Poll::Pending)) {
            return Ok(());
        }
        self.workspace_autosave_pending = None;
        PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
        let result = if current {
            result
        } else {
            Ok(save_control::Poll::Stale)
        };
        match result? {
            save_control::Poll::Ready(prepared) => {
                if !save_snapshot::automatic_ready(self)? {
                    return Ok(());
                }
                let published = prepared.commit_ready(self, &AtomicBool::new(false))?;
                if let Some((path, error)) = published.retained_recovery {
                    return Err(Error::new(
                        E_FAIL,
                        crate::ui_text::format(
                            "app-workspace-recovery-retained",
                            &[
                                ("path", path.display().to_string().into()),
                                ("error", error.to_string().into()),
                            ],
                        ),
                    ));
                }
            }
            // Edits normally invalidate old snapshots; save only the current snapshot at the next idle point, without replaying an old candidate.
            save_control::Poll::Stale => {}
            save_control::Poll::Pending => unreachable!(),
        }
        Ok(())
    }
    pub(super) fn automatic_save_pending(&self) -> bool {
        self.workspace_autosave_pending.is_some() || self.checkpoint_pending()
    }
    pub(super) unsafe fn workspace_autosave_blocks_exit(&self) -> bool {
        self.workspace_autosave_pending.is_some()
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "application_workspace_autosave_tests.rs"]
mod tests;
