//! Sender results retain original four-slot/lease/same-intent activity permits; no retirement of write-ahead records before persistent workspace binding.
use super::*;
use crate::projection_delivery::sender::{Active, Request, Response};
pub(super) struct Connected {
    pub active: Active,
    pub response: Response,
    pub candidate: Option<crate::projection_delivery::sender::binding::Candidate>,
}
pub struct Ready {
    pub(super) value: Connected,
    _slot: Slot,
}
impl Ready {
    pub(super) fn new(value: Connected, slot: Slot) -> Self {
        Self { value, _slot: slot }
    }
    pub fn response(&self) -> &Response {
        &self.value.response
    }
    pub fn request(&self) -> Request {
        self.value.active.request().clone()
    }
    pub fn validate(&self, current: &Lease, now: u64) -> Result<&Response, Error> {
        let active = &self.value.active;
        active.check().map_err(Error::Sender)?;
        current
            .validate(active.intent().envelope().origin(), now)
            .map_err(|_| Error::Stopped)?;
        if !active.lease().same_grant(current) {
            return Err(Error::Stopped);
        }
        Ok(&self.value.response)
    }
    /// Background only: full outbox baselines/original-session patches may require large I/O/encoding, never UI menu execution.
    /// Returned candidates preserve original journals; callers still recheck source/workspace/authorization and await their exact checkpoint commit.
    pub fn prepare_binding(
        &self,
        source: &crate::session_source::Source,
        name: &str,
        current: &Lease,
        now: u64,
    ) -> Result<crate::projection_delivery::sender::binding::Candidate, Error> {
        let response = self.validate(current, now)?;
        let candidate = crate::projection_delivery::sender::binding::Binding::prepare(
            source,
            self.request().intent(),
            response,
            name,
            current.preparation_cancel(),
        )
        .map_err(Error::Sender)?;
        self.validate(current, now)?;
        Ok(candidate)
    }
    pub fn request_stop(&self) {
        self.value.active.lease().revoke();
    }
}
impl std::fmt::Debug for Ready {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.response.fmt(f)
    }
}
