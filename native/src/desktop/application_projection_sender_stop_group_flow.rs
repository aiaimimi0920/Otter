//! Durably accept entries one by one, then drain through original fair stop scheduling; cancellation drops only unaccepted entries, and durable requests recover across instances.
use super::*;
impl Application {
    pub(in crate::desktop) unsafe fn start_projection_stop_group(
        &mut self,
        selection: StopSelection,
    ) {
        if let Some(batch) = Batch::capture(self, selection) {
            self.stop_projection_sender();
            self.projection_sender.stop_group = Some(batch);
            self.process_projection_stop_group();
        } else {
            self.sender_status(crate::ui_text::text("app-projection-stop-group-changed"));
        }
    }
    pub(in crate::desktop::application::projection_sender) fn cancel_projection_stop_group(
        &mut self,
    ) {
        if let Some(mut batch) = self.projection_sender.stop_group.take() {
            batch.cancel();
            batch.report(self);
        }
    }
    pub(in crate::desktop::application::projection_sender) unsafe fn process_projection_stop_group(
        &mut self,
    ) {
        let Some(mut batch) = self.projection_sender.stop_group.take() else {
            return;
        };
        if !batch.matches(self) || self.exit_requested.get() {
            batch.cancel();
            batch.report(self);
            return;
        }
        for entry in &mut batch.entries {
            let before = entry.state;
            if let Some(request) = &entry.request {
                if request.published_retired()
                    && self
                        .workspace_checkpoint
                        .as_ref()
                        .and_then(|owner| owner.sender_index())
                        .is_some_and(|index| {
                            index.is_for(&batch.canonical)
                                && index.for_intent(request.intent()).is_none()
                        })
                {
                    entry.state = State::Stopped;
                } else if request.stop_requested() {
                    entry.state = State::Pending;
                } else if entry.state == State::Starting && !self.projection_sender.stop.pending() {
                    entry.state = State::Failed;
                }
            }
            batch.dirty |= before != entry.state;
        }
        if !self.projection_sender.stop.pending() && self.sender_stop_available() {
            while let Some(entry) = batch
                .entries
                .iter_mut()
                .find(|entry| entry.state == State::Queued)
            {
                // Accept only original associations in the fixed canonical; capture already checked Request/identity, which carries no network authorization.
                if let Some(request) = entry
                    .request
                    .as_ref()
                    .filter(|_| self.projection_sender_inclusion(&entry.binding))
                {
                    self.begin_sender_stop_request(request.clone());
                    entry.state = if self.projection_sender.stop.pending() {
                        State::Starting
                    } else {
                        State::Failed
                    };
                } else {
                    entry.state = State::Failed;
                }
                batch.dirty = true;
                if self.projection_sender.stop.pending() {
                    break;
                }
            }
        }
        batch.report(self);
        if batch.entries.iter().any(|entry| {
            matches!(
                entry.state,
                State::Queued | State::Starting | State::Pending
            )
        }) {
            self.projection_sender.stop_group = Some(batch);
        } else {
            self.sender_status(crate::ui_text::text("app-projection-stop-group-complete"));
        }
    }
}
