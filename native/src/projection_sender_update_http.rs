//! Continuous sending reads existing IDs then CAS; uncertain updates reconcile with one read, never resend/Create/broaden cancellation.
use super::*;
use delivery::sender::{binding::Binding, update::Update};
impl Control<'_> {
    pub(crate) fn read_sender_update(
        &self,
        active: &Active,
        expected: &Binding,
    ) -> Result<Response, Error> {
        let body = serde_json::to_vec(&serde_json::json!({
            "projectionId":active.intent().envelope().projection_id(),
            "knownRevision":expected.revision()
        }))
        .map_err(|_| Error::Input)?;
        let response = self
            .sender_exchange(active, active.intent().path(Operation::Read), &body, false)?
            .ok_or(Error::Status(404))?;
        delivery::sender::update::validate(expected, active.intent(), &response)
            .map_err(Error::Sender)?;
        Ok(response)
    }
    pub(crate) fn update_sender(
        &self,
        active: &Active,
        expected: &Binding,
        update: &Update,
    ) -> Result<Response, Error> {
        let body = update.body(active).map_err(Error::Sender)?;
        let result = self
            .sender_exchange(
                active,
                active.intent().path(Operation::Update),
                &body,
                false,
            )
            .and_then(|response| response.ok_or(Error::Input))
            .and_then(|response| {
                if update.matches(&response) {
                    Ok(response)
                } else {
                    Err(Error::Sender(delivery::sender::Error::Response))
                }
            });
        match result {
            Ok(response) => Ok(response),
            Err(original) => {
                active.check().map_err(Error::Sender)?;
                match self.read_sender_update(active, expected) {
                    Ok(response) if update.matches(&response) => Ok(response),
                    _ => Err(original),
                }
            }
        }
    }
}
