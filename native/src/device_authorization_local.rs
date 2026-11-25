//! Unverified stop lease for local Art only; no Session/Device token, and all Control/DNS preparation is rejected.
use super::*;
use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};
pub(super) struct Credential {
    origin: String,
    accepted: OnceLock<Accepted>,
}
struct Accepted {
    device: String,
    expires: u64,
    observed: Mutex<u64>,
    deadline: Instant,
}
impl Credential {
    pub(super) fn device(&self) -> &str {
        self.accepted.get().map_or("", |a| a.device.as_str())
    }
    pub(super) fn expires(&self) -> u64 {
        self.accepted.get().map_or(0, |a| a.expires)
    }
    pub(super) fn validate(&self, origin: &str, now: u64) -> Result<(), Error> {
        let accepted = self.accepted.get().ok_or(Error::Credential)?;
        let mut observed = accepted.observed.lock().map_err(|_| Error::Credential)?;
        if origin != self.origin
            || now < *observed
            || now >= accepted.expires
            || Instant::now() >= accepted.deadline
        {
            return Err(Error::Credential);
        }
        *observed = now;
        Ok(())
    }
}
impl Lease {
    /// UI creates only cancellation/wake state; validate always fails until background signatures/identity/persistent-clock gates pass.
    pub(crate) fn pending_local(origin: &str) -> Self {
        Self {
            session: None,
            local: Some(Box::new(Credential {
                origin: origin.into(),
                accepted: OnceLock::new(),
            })),
            state: Arc::new(State {
                cancel: Cancel::new(),
                execution_cancel: Cancel::new(),
                execution_watch: watch::Watch::default(),
                media: Mutex::new(None),
                watch: watch::Watch::default(),
            }),
        }
    }
    pub(crate) fn is_local(&self) -> bool {
        self.local.is_some()
    }
    /// Callers must first verify Ticket signature/current identity/source and commit the persistent clock high-water mark; activation is one-time.
    pub(crate) fn accept_local(
        &self,
        ticket: &crate::art_execution_grant::resume::Ticket,
        now: u64,
    ) -> Result<(), Error> {
        let local = self.local.as_ref().ok_or(Error::Credential)?;
        let expires = ticket
            .grant
            .expires_at_ms
            .min(ticket.grant.offline_until_ms);
        if self.execution_stopped() || local.origin != ticket.grant.binding.origin || now >= expires
        {
            return Err(Error::Credential);
        }
        local
            .accepted
            .set(Accepted {
                device: ticket.grant.binding.device_id.clone(),
                expires,
                observed: Mutex::new(now),
                deadline: Instant::now() + Duration::from_millis(expires - now),
            })
            .map_err(|_| Error::Bound)?;
        self.validate(&local.origin, now)
    }
}
