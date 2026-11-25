//! One-shot Surface click submission, background-only; errors never trigger replay or fallback to administrator credentials.
use super::*;
use crate::{
    surface_action_ack::{Ack, MAX_ACK_BYTES},
    surface_click_event::ClickEvent,
    surface_lifecycle_owner::Owner,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    AlreadyAttempted,
    Inactive,
    Stale,
    Identity,
    Transport(Error),
    Ack(crate::surface_action_ack::Error),
}
impl Control<'_> {
    /// current_revision must come from the current presentation owner, not the event's own revision masquerading as a fresh check.
    /// After attempted=true, no matching ACK means unconfirmed; retain the event for subsequent query convergence.
    pub fn submit_surface_click(
        &self,
        owner: &Owner,
        current_revision: u64,
        event: &mut ClickEvent,
    ) -> Result<Ack, SubmitError> {
        self.submit_surface_click_with_timeout(
            owner,
            current_revision,
            event,
            Duration::from_secs(10),
        )
    }
    pub(super) fn submit_surface_click_with_timeout(
        &self,
        owner: &Owner,
        current_revision: u64,
        event: &mut ClickEvent,
        timeout: Duration,
    ) -> Result<Ack, SubmitError> {
        if event.attempted() {
            return Err(SubmitError::AlreadyAttempted);
        }
        if !owner.confirmed_active() {
            return Err(SubmitError::Inactive);
        }
        if owner.binding() != event.binding() || current_revision != event.revision() {
            return Err(SubmitError::Stale);
        }
        let binding = owner.binding();
        if binding.origin() != self.client.origin()
            || binding.device() != self.session.device_id()
            || matches!(binding.instance(), "." | "..")
        {
            return Err(SubmitError::Identity);
        }
        let mut url =
            url::Url::parse("https://path.invalid/").map_err(|_| SubmitError::Identity)?;
        url.path_segments_mut()
            .map_err(|_| SubmitError::Identity)?
            .clear()
            .extend(["v1", "surfaces", "instances", binding.instance(), "events"]);
        // Conservatively prohibit resubmission from here onward; even without authentication/network delivery confirmation, never guess server-side effects.
        event.begin_submission();
        let bytes = self
            .send_limited("POST", url.path(), event.bytes(), timeout, MAX_ACK_BYTES)
            .map_err(SubmitError::Transport)?;
        Ack::parse(&bytes, event).map_err(SubmitError::Ack)
    }
}
