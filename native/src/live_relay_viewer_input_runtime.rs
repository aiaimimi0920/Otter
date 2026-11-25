//! Reuse the original serial control thread for Surface input; no unknown-HTTP-result replay, best-effort exit release without falsely claiming source execution.
use super::*;
use crate::{
    live_input::MAX_SEQUENCE,
    live_relay_http::{input::Binding, Control},
};
use std::time::Instant;
#[path = "live_relay_viewer_input_queue.rs"]
mod queue;
pub(crate) use queue::{Queue, Status};
#[path = "art_live_trigger_driver.rs"]
mod local_trigger;
pub(super) struct Sender {
    queue: Arc<Queue>,
    cursor: RequesterCursor,
    owned: bool,
    renew: Instant,
    accepted: u64,
    local_poll: Instant,
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_relay_viewer_input_runtime_tests.rs"]
mod tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_relay_viewer_trigger_runtime_tests.rs"]
mod trigger_tests;
pub(super) struct Finish(pub Arc<Queue>);
impl Drop for Finish {
    fn drop(&mut self) {
        self.0.finish(false);
    }
}
impl Sender {
    pub fn new(queue: Arc<Queue>, cursor: RequesterCursor) -> Self {
        Self {
            queue,
            cursor,
            owned: false,
            renew: Instant::now(),
            accepted: 0,
            local_poll: Instant::now(),
        }
    }
    pub fn active(&self) -> bool {
        self.queue.intent().0
            || self.queue.triggers.phase() == crate::live_relay_viewer_trigger_queue::Phase::Pending
            || self.queue.local_triggers.command_pending()
    }
    pub fn drive_commands(
        &mut self,
        control: &Control<'_>,
        target: &RequestTarget,
        owner: &mut Owner,
    ) -> Result<bool, HttpError> {
        if matches!(target, RequestTarget::Direct(_)) {
            return Ok(self.queue.intent().1);
        }
        // Privilege-reducing release precedes potentially slow configuration HTTP; old keys must not wait for configuration reconciliation to release.
        if self.drive(control, target, owner.identity(), owner.after())? {
            return Ok(true);
        }
        match target {
            RequestTarget::Surface(_) => self.drive_trigger(control, owner)?,
            RequestTarget::LocalArt(target) => self.drive_local_trigger(control, target, owner)?,
            RequestTarget::Direct(_) => unreachable!("media-only viewer"),
        }
        Ok(false)
    }
    /// Triggers and keyboard/mouse share one actor cursor; no second sending thread or injection of GET observation snapshots into reliable event streams.
    pub fn drive_trigger(
        &mut self,
        control: &Control<'_>,
        owner: &mut Owner,
    ) -> Result<(), HttpError> {
        use crate::live_relay_viewer_trigger_queue::Phase;
        let Some(command) = self.queue.triggers.take() else {
            return Ok(());
        };
        let result = control.configure_trigger(owner.identity(), self.cursor, &command);
        match result {
            Ok(outcome) => {
                if outcome.accepted {
                    self.cursor.control = self.next()?;
                }
                if self.queue.triggers.complete(if outcome.accepted {
                    Phase::Confirmed
                } else {
                    Phase::Rejected
                }) {
                    owner
                        .registrations(outcome.registrations)
                        .map_err(|_| HttpError::Cursor)?;
                }
                self.queue.notify_trigger();
                Ok(())
            }
            Err(error) => {
                self.queue.triggers.complete(Phase::Unknown);
                self.queue.notify_trigger();
                Err(error)
            }
        }
    }
    pub fn observe(&mut self, owned: bool) {
        if self.owned && !owned {
            self.queue.release();
        }
        self.owned &= owned;
        self.queue.acknowledge(self.owned);
    }
    fn next(&self) -> Result<u64, HttpError> {
        self.cursor
            .control
            .checked_add(1)
            .filter(|n| *n <= MAX_SEQUENCE)
            .ok_or(HttpError::Cursor)
    }
    pub fn drive(
        &mut self,
        control: &Control<'_>,
        target: &RequestTarget,
        identity: &Identity,
        after: u64,
    ) -> Result<bool, HttpError> {
        if matches!(target, RequestTarget::LocalArt(target) if target.proof.validate().is_err()) {
            // Ready viewers outlive their launching host, but control must not. Release is a
            // reduction-only endpoint even after host expiry/reset; watching remains independent.
            self.queue.retire_host();
        }
        for _ in 0..16 {
            let (requested, stopping) = self.queue.intent();
            if !requested || self.queue.releasing() {
                if self.owned {
                    let sequence = self.next()?;
                    target.change_controller(control, identity, sequence, false)?;
                    self.cursor.control = sequence;
                    self.owned = false;
                    self.queue.acknowledge(false);
                }
                return Ok(stopping);
            }
            if !self.owned || Instant::now() >= self.renew {
                let sequence = self.next()?;
                self.queue.mark_possible();
                target.change_controller(control, identity, sequence, true)?;
                self.cursor.control = sequence;
                self.owned = true;
                self.renew = Instant::now() + Duration::from_secs(15);
                self.queue.acknowledge(true);
                continue;
            }
            let Some(event) = self.queue.take() else {
                return Ok(false);
            };
            let cursor = RequesterCursor {
                control: self.next()?,
                input: self
                    .cursor
                    .input
                    .checked_add(1)
                    .filter(|n| *n <= MAX_SEQUENCE)
                    .ok_or(HttpError::Cursor)?,
            };
            let accepted = target.forward_input(control, identity, cursor, event)?;
            if accepted <= after.max(self.accepted) {
                return Err(HttpError::Cursor);
            }
            self.cursor = cursor;
            self.accepted = accepted;
        }
        Ok(false)
    }
}
impl RequestTarget {
    fn change_controller(
        &self,
        control: &Control<'_>,
        identity: &Identity,
        sequence: u64,
        acquire: bool,
    ) -> Result<(), HttpError> {
        match self {
            Self::Surface(t) => control.change_viewer_controller(
                identity,
                Binding {
                    surface: &t.surface,
                    attachment: &t.attachment,
                },
                sequence,
                acquire,
            ),
            Self::LocalArt(t) => {
                control.change_local_art_controller(identity, &t.proof, sequence, acquire)
            }
            Self::Direct(_) => Err(HttpError::Input),
        }
    }
    fn forward_input(
        &self,
        control: &Control<'_>,
        identity: &Identity,
        cursor: RequesterCursor,
        event: crate::live_input::Event,
    ) -> Result<u64, HttpError> {
        match self {
            Self::Surface(t) => control.forward_window_input(
                identity,
                Binding {
                    surface: &t.surface,
                    attachment: &t.attachment,
                },
                cursor,
                event,
            ),
            Self::LocalArt(t) => control.forward_local_art_input(identity, &t.proof, cursor, event),
            Self::Direct(_) => Err(HttpError::Input),
        }
    }
}
