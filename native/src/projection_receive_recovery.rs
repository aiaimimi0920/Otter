//! Read-only verification for explicit recovery; original Ticket permits span read/pixel verification/Ready without rereceive or local-image mutation.
use super::*;
use crate::projection_delivery::{binding::Binding, journal::Ticket};

pub(super) struct Request {
    pub ticket: Ticket,
    pub expected: Binding,
    pub image: crate::image_decode::Image,
}
impl Request {
    pub(super) fn run(
        self,
        control: &crate::live_relay_http::Control<'_>,
        client: &Client,
        lease: &Lease,
        attempt: &attempt::Attempt,
    ) -> Result<Completed, Error> {
        if self.ticket.cleanup_pending()
            || self.ticket.device() != lease.device_id()
            || self.ticket.origin() != client.origin()
            || !self.expected.matches_ticket(&self.ticket)
        {
            return Err(Error::Stopped);
        }
        // Original Job holds permitted Ticket; workers use only permit-free snapshots, and errors/unwinding never release permits early.
        let ticket = attempt.recover(self.ticket)?;
        let response = control.read_projection(&ticket).map_err(Error::Http)?;
        let value = received::decode(response, lease, &ticket)?;
        // Ordinary saving re-encodes PNG; compare decoded original pixels, not digests of saved compressed bytes.
        if !crate::projection_delivery::binding::Binding::new(&value, &ticket)
            .map_err(Error::Decode)?
            .same_snapshot(&self.expected)
            || value.image().width != self.image.width
            || value.image().height != self.image.height
            || value.image().pixels != self.image.pixels
            || lease.is_revoked()
        {
            return Err(Error::Stopped);
        }
        Ok(Completed::Accepted(Box::new(value)))
    }
}
