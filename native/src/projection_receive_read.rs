//! Monotonic snapshot contract for continuous read; unchanged still validates complete original associations and grants no display receipt.
use super::*;
use crate::projection_delivery::{binding::Binding, journal::Ticket};

#[derive(Debug)]
pub(crate) enum Read {
    Unchanged,
    Editing(crate::projection_edit::Document),
    Changed(Box<Received>),
}
impl Read {
    pub(crate) fn parse(
        bytes: &[u8],
        ticket: &Ticket,
        expected: &Binding,
        origin: &str,
        device: &str,
    ) -> Result<Self, Error> {
        if bytes.len() > MAX_RESPONSE {
            return Err(Error::Limit);
        }
        if !expected.matches_ticket(ticket) || ticket.device() != device || ticket.cleanup_pending()
        {
            return Err(Error::Response);
        }
        let invitation = ticket.invitation();
        invitation.validate_for(origin)?;
        let wire: Wire<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        wire.envelope.validate(ticket.envelope().origin())?;
        if &wire.envelope != ticket.envelope()
            || !invitation.response_route(
                wire.route,
                wire.peer_id,
                wire.offline_transport,
                wire.delivery,
                device,
            )
            || !wire.linked
            || wire.receiver_device_id != device
            || wire.receiver_unit_id != ticket.unit()
            || !(expected.revision()..=envelope::MAX_REVISION).contains(&wire.revision)
            || !envelope::digest(&wire.digest)
            || wire.width == 0
            || wire.height == 0
            || wire.width > 8192
            || wire.height > 8192
            || u64::from(wire.width) * u64::from(wire.height) > MAX_PIXELS
        {
            return Err(Error::Response);
        }
        if wire.revision == expected.revision() {
            if wire.digest != expected.digest() {
                return Err(Error::Response);
            }
            // Same-version replies containing images still fully validate PNG, never bypassing budget/digest checks.
            if wire.snapshot.is_some() {
                return Received::from_wire(wire, invitation.offline)
                    .map(|value| Self::Changed(Box::new(value)));
            }
            return Ok(
                match edit::parse(wire.editing, &wire.digest, wire.width, wire.height)? {
                    Some(document) => Self::Editing(document),
                    None => Self::Unchanged,
                },
            );
        }
        Received::from_wire(wire, invitation.offline).map(|value| Self::Changed(Box::new(value)))
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_receive_read_tests.rs"]
mod tests;
