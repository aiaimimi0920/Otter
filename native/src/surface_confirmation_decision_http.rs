//! Bounded confirmation-decision POST; no retries, administrator-interface fallback, or treating HTTP success as action success.
use super::*;
use crate::{
    surface_action_ack::Ack, surface_click_event::ClickEvent, surface_confirmation::Decision,
    surface_lifecycle_owner::Owner,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionSubmitError {
    AlreadyAttempted,
    Stopped,
    Inactive,
    Stale,
    Identity,
    Request(crate::surface_confirmation::Error),
    Transport(Error),
}
impl Control<'_> {
    pub fn submit_surface_confirmation(
        &self,
        owner: &Owner,
        current_revision: u64,
        event: &ClickEvent,
        ack: &Ack,
        decision: &mut Decision,
    ) -> Result<(), DecisionSubmitError> {
        if decision.attempted() {
            return Err(DecisionSubmitError::AlreadyAttempted);
        }
        if self.cancel.is_stopped() {
            return Err(DecisionSubmitError::Stopped);
        }
        if !owner.confirmed_active() {
            return Err(DecisionSubmitError::Inactive);
        }
        if owner.binding() != event.binding() || current_revision != event.revision() {
            return Err(DecisionSubmitError::Stale);
        }
        if owner.binding().origin() != self.client.origin()
            || owner.binding().device() != self.session.device_id()
            || !decision.matches(event, ack)
        {
            return Err(DecisionSubmitError::Identity);
        }
        let now = now().map_err(DecisionSubmitError::Transport)?;
        decision
            .request()
            .validate(event, ack, now)
            .map_err(DecisionSubmitError::Request)?;
        let timeout = Duration::from_millis(decision.request().expires_at_ms() - now)
            .min(Duration::from_secs(10));
        // Network errors beyond this point leave the outcome unconfirmed; never resubmit automatically or change approval to rejection and resend.
        decision.begin_submission();
        self.send_limited(
            "POST",
            "/v1/surfaces/confirmations/decision",
            decision.bytes(),
            timeout,
            64 * 1024,
        )
        .map_err(DecisionSubmitError::Transport)?;
        decision.acknowledge();
        Ok(())
    }
}
