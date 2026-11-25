//! Only the application loop advances group queues; original Retry blocks later entries, and explicit cancellation does not roll back confirmed send associations.
use super::*;
use crate::device_authorization_config::Config;
impl Application {
    pub(in crate::desktop) unsafe fn start_projection_group(
        &mut self,
        selection: Selection,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        match Batch::capture(self, selection) {
            Ok(batch) => {
                self.projection_sender.group = Some(batch);
                self.process_projection_group(config);
            }
            Err(_) => self.sender_status(crate::ui_text::text("app-projection-group-unavailable")),
        }
    }
    pub(in crate::desktop::application::projection_sender) fn cancel_projection_group(&mut self) {
        if let Some(mut batch) = self.projection_sender.group.take() {
            if let Phase::Checkpoint(saved) = &self.projection_sender.phase {
                if self.projection_sender_inclusion(&saved.expected) {
                    batch.committed(saved);
                }
            }
            batch.cancel();
            batch.report(self);
        }
    }
    pub(in crate::desktop::application::projection_sender) unsafe fn process_projection_group(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let Some(mut batch) = self.projection_sender.group.take() else {
            return;
        };
        // Waiting/Checkpoint checks source through the original owner; source content published by this batch but not yet persisted is not external replacement.
        if !matches!(self.projection_sender.phase, Phase::Idle) {
            batch.report(self);
            self.projection_sender.group = Some(batch);
            return;
        }
        if let Some(index) = batch.current.take() {
            batch.entries[index].state = State::Failed;
            batch.cancel();
        }
        if !batch.matches(self) {
            batch.cancel();
        }
        let next = batch
            .entries
            .iter()
            .position(|entry| entry.state == State::Queued);
        if let Some(index) = next {
            if self.projection_sender.pending_local()
                || self.projection_sender.cleanup.network.is_some()
                || self.remote_sessions.projection_pending()
                || self.projection_receive.busy()
            {
                batch.report(self);
                self.projection_sender.group = Some(batch);
                return;
            }
            let started = batch.admission().is_ok_and(|admission| {
                self.start_projection_sender_admitted(
                    crate::desktop::device_authorization_ui::sender::Selection {
                        unit: batch.selection.unit.clone(),
                        directory: batch.selection.directory.clone(),
                        index: batch.entries[index].index,
                    },
                    admission,
                    config,
                )
            });
            batch.entries[index].state = if started {
                State::Sending
            } else {
                State::Failed
            };
            batch.dirty = true;
            if started {
                batch.current = Some(index);
            } else {
                batch.cancel();
            }
        }
        batch.report(self);
        if batch.current.is_some() {
            self.projection_sender.group = Some(batch);
        } else {
            self.sender_status(crate::ui_text::text("app-projection-group-complete"));
        }
    }
}
