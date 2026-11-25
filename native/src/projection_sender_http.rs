//! Same-ID read -> explicit absence -> Create; after lost replies, read once more without resending Create/changing signed identity.
use super::*;
use delivery::sender::{Active, Operation, Response};
#[path = "projection_sender_update_http.rs"]
mod update;
impl Control<'_> {
    pub(crate) fn send_projection(&self, active: &Active) -> Result<Response, Error> {
        if let Some(response) = self.read_sender(active)? {
            return Ok(response);
        }
        let body = active.create_body().map_err(Error::Sender)?;
        let result = self
            .sender_exchange(active, active.intent().path(Operation::Create), &body, true)
            .and_then(|response| response.ok_or(Error::Input));
        match result {
            Ok(response) => Ok(response),
            Err(original) => {
                // Includes rejection/bad responses/uncertain HTTP; one read-only reconciliation adds no remote side effects.
                active.check().map_err(Error::Sender)?;
                match self.read_sender(active) {
                    Ok(Some(response)) => Ok(response),
                    _ => Err(original),
                }
            }
        }
    }
    fn read_sender(&self, active: &Active) -> Result<Option<Response>, Error> {
        let body = active.read_body().map_err(Error::Sender)?;
        self.sender_exchange(active, active.intent().path(Operation::Read), &body, false)
    }
    fn sender_exchange(
        &self,
        active: &Active,
        path: &'static str,
        body: &[u8],
        created: bool,
    ) -> Result<Option<Response>, Error> {
        active.check().map_err(Error::Sender)?;
        if active.intent().envelope().origin() != self.client.origin()
            || active.intent().envelope().source_device() != self.session.device_id()
        {
            return Err(Error::Input);
        }
        let limit = delivery::accept::MAX_RESPONSE;
        let mut budget = crate::pixels::reserve(limit.next_power_of_two())
            .ok_or(Error::Sender(delivery::sender::Error::Limit))?;
        let (status, bytes) = self.exchange_projection_until(
            path,
            body,
            Instant::now() + Duration::from_secs(12),
            limit,
        )?;
        if !budget.grow_to(bytes.capacity()) {
            return Err(Error::Sender(delivery::sender::Error::Limit));
        }
        active.check().map_err(Error::Sender)?;
        self.session
            .token_for(self.client.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        if status == 401 {
            return Err(Error::AuthorizationRequired);
        }
        if !(200..300).contains(&status) {
            // Ordinary 404, gateway bodies, other business codes, and all other HTTP statuses do not authorize creation.
            if !created && status == 404 && error_code(&bytes, "projection_not_found") {
                return Ok(None);
            }
            return Err(Error::Status(status));
        }
        Response::parse(
            &bytes,
            active.intent(),
            created,
            active.lease().preparation_cancel(),
        )
        .map(Some)
        .map_err(Error::Sender)
    }
    pub(crate) fn unlink_sender(&self, active: &Active) -> Result<(), Error> {
        active.check().map_err(Error::Sender)?;
        if active.intent().envelope().source_device() != self.session.device_id() {
            return Err(Error::Input);
        }
        self.unlink_projection_at(
            active.intent().envelope(),
            active.intent().path(Operation::Unlink),
        )?;
        active.retire().map_err(Error::Sender)
    }
}
