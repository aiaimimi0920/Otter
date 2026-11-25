//! One-time registration plus exact read-only reconciliation; no unknown-result replay or overwriting reliable observation cursors with newer business snapshots.
use super::*;
use crate::{
    live_relay_trigger_command::Command,
    live_relay_trigger_registration::{self as registration, Registration},
};
pub(crate) struct TriggerOutcome {
    pub accepted: bool,
    pub registrations: Vec<Registration>,
}
impl Control<'_> {
    pub(crate) fn trigger_snapshot(
        &self,
        identity: &Identity,
    ) -> Result<(RequesterCursor, Vec<Registration>), Error> {
        let bytes = self.send(
            "GET",
            &path(identity.session(), None)?,
            &[],
            Duration::from_secs(3),
        )?;
        let snapshot = Snapshot::parse(&bytes, identity.session()).map_err(Error::Protocol)?;
        snapshot
            .renewal(identity, self.session.device_id())
            .map_err(Error::Protocol)?;
        let cursor = join::recovery(&bytes, &snapshot, self.session.device_id())?;
        let registrations = registration::from_snapshot(&bytes).map_err(|_| Error::Input)?;
        Ok((cursor, registrations))
    }
    pub(crate) fn configure_trigger(
        &self,
        identity: &Identity,
        baseline: RequesterCursor,
        command: &Command,
    ) -> Result<TriggerOutcome, Error> {
        command.validate().map_err(|_| Error::Input)?;
        let next = baseline.next_control()?;
        if next > crate::live_input::MAX_SEQUENCE {
            return Err(Error::Cursor);
        }
        let (before, registrations) = self.trigger_snapshot(identity)?;
        if before != baseline {
            return Err(Error::Cursor);
        }
        let revision = registrations
            .iter()
            .find(|r| r.binding.binding_id == command.binding_id)
            .map_or(Some(1), |r| r.condition.revision.checked_add(1));
        if revision != Some(command.condition.revision) {
            return Ok(TriggerOutcome {
                accepted: false,
                registrations,
            });
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "bindingId":command.binding_id,"enabled":command.enabled,"target":command.target,
            "envelope":{"protocolVersion":protocol::PROTOCOL,"sessionId":identity.session(),
                "epoch":identity.epoch(),"sequence":next,"messageType":"trigger_condition",
                "payload":command.condition}
        }))
        .map_err(|_| Error::Input)?;
        let result = self.send(
            "POST",
            &path(identity.session(), Some("triggers"))?,
            &body,
            Duration::from_secs(3),
        );
        // POST responses omit requesterControl; regardless of response loss, read only our own cursor and the complete unique registration.
        let (after, registrations) = self.trigger_snapshot(identity)?;
        let exact = registrations
            .iter()
            .any(|r| command.matches(r, self.session.device_id()));
        if after
            == (RequesterCursor {
                control: next,
                input: baseline.input,
            })
            && exact
        {
            return Ok(TriggerOutcome {
                accepted: true,
                registrations,
            });
        }
        if matches!(result, Err(Error::Status(400..=499))) && after == baseline {
            return Ok(TriggerOutcome {
                accepted: false,
                registrations,
            });
        }
        // Old cursors observed after timeout do not prove in-flight POST will never commit; close old viewing rather than unsafely reuse the actor.
        Err(result.err().unwrap_or(Error::Cursor))
    }
}
