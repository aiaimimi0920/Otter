//! Cancellation handoff for authorization/source/workspace/exit; ordinary exit or workspace changes must not bulk-unlink stable persisted bindings.
use super::*;
impl Application {
    pub(in crate::desktop) fn stop_projection_sender(&mut self) {
        self.cancel_projection_group();
        self.cancel_projection_stop_group();
        self.projection_sender.updates.clear();
        let phase = std::mem::take(&mut self.projection_sender.phase);
        match phase {
            Phase::Waiting(mut pending) => {
                pending.stopped = true;
                self.remote_sessions.cancel_projection();
                self.projection_sender.phase = Phase::Waiting(pending);
            }
            Phase::Checkpoint(saved) => self.cancel_sender_saved(saved),
            Phase::Updating(mut pending) => {
                pending.stopped = true;
                self.remote_sessions.cancel_projection();
                self.projection_sender.phase = Phase::Updating(pending);
            }
            Phase::Retry(pending) => {
                if let Some(request) = pending.request {
                    self.projection_sender.cleanup.enqueue(request, false);
                }
            }
            Phase::Idle => {}
        }
    }
    pub(super) fn sender_saved_current(&self, saved: &Saved) -> bool {
        saved.workspace.matches(&self.workspace)
            && self.restore_job.is_none()
            && self.projection_checkpoint_available()
            && self
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == saved.expected.unit())
            && self.sender_current(&saved.current, saved.expected.envelope().origin())
    }
    pub(super) fn cancel_sender_saved(&mut self, saved: Box<Saved>) {
        if saved.retained_disk || self.projection_sender_inclusion(&saved.expected) {
            return;
        }
        let request = saved.ready.request();
        let same_workspace = saved.workspace.matches(&self.workspace);
        let rollback = self
            .restored_session
            .as_ref()
            .is_some_and(|record| record.original.same_storage(&saved.source));
        if same_workspace && !rollback {
            // Another transaction changed the complete source document, so absence of a saved binding cannot be proven; retain the original log for explicit recovery instead of guessing an unlink.
            return;
        }
        if same_workspace {
            self.restored_session
                .as_mut()
                .unwrap()
                .edit_original(saved.previous.clone());
            self.saved_state = None;
        }
        saved.ready.request_stop();
        drop(saved); // Release Active/Slot first so later CancelSend can activate again.
        self.projection_sender.cleanup.enqueue(request, false);
    }
    pub(in crate::desktop) fn flush_projection_sender(&mut self) -> Result<()> {
        self.stop_projection_sender();
        self.remote_sessions.finish_projection();
        self.collect_projection_sender();
        if let Phase::Waiting(pending) = std::mem::take(&mut self.projection_sender.phase) {
            if let Some(request) = pending.request {
                self.projection_sender.cleanup.enqueue(request, false);
            }
        }
        self.projection_sender.profile.collect(true);
        self.projection_sender
            .cleanup
            .resolve(&self.projection_sender.profile);
        self.projection_sender.cleanup.flush().map_err(|error| {
            Error::new(
                E_FAIL,
                format!("Sender cleanup remains unpersisted: {error:?}"),
            )
        })
    }
}
