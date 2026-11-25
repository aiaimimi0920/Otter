//! Viewer business-state transactions: validate whole observation/audit batches before replacement, without Source input or Surface actions.
use crate::{
    live_relay_observation::{self as observation, Capabilities, Observations},
    live_relay_trigger_audit::{self as audit, Audits},
    live_relay_trigger_registration::{self as registration, Registration},
    live_relay_viewer_identity::ControlEvent,
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Interaction,
    Observation(observation::Error),
    Audit(audit::Error),
    Registration(registration::Error),
}
#[derive(Clone)]
pub struct Business {
    display_identity: Arc<()>,
    interaction: crate::live_relay_http::input::Capabilities,
    capabilities: Capabilities,
    observations: Observations,
    audits: Audits,
    registrations: Arc<[Registration]>,
}
impl Business {
    /// Must come from the same validated attach response; catalog responses clear business data and cannot replace attach.
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            display_identity: Arc::new(()),
            interaction: crate::live_relay_http::input::Capabilities::parse(bytes)
                .map_err(|_| Error::Interaction)?,
            capabilities: Capabilities::from_snapshot(bytes).map_err(Error::Observation)?,
            observations: Observations::from_snapshot(bytes).map_err(Error::Observation)?,
            audits: Audits::from_snapshot(bytes).map_err(Error::Audit)?,
            registrations: registration::from_snapshot(bytes)
                .map_err(Error::Registration)?
                .into(),
        })
    }
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
    pub(crate) fn interaction(&self) -> crate::live_relay_http::input::Capabilities {
        self.interaction
    }
    pub fn observations(&self) -> &Observations {
        &self.observations
    }
    pub fn audits(&self) -> &Audits {
        &self.audits
    }
    pub fn registrations(&self) -> &[Registration] {
        &self.registrations
    }
    pub(crate) fn replace_registrations(&mut self, registrations: Vec<Registration>) {
        if self.registrations.as_ref() != registrations.as_slice() {
            self.registrations = registrations.into();
            self.display_identity = Arc::new(());
        }
    }
    /// Input/session heartbeats do not invalidate read-only semantic projections; identity advances only in committed semantic transactions.
    pub(crate) fn same_display(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.display_identity, &other.display_identity)
    }
    /// Callers must first validate envelope session/epoch/continuous sequence and commit the control cursor only after success.
    pub fn apply(&mut self, events: &[ControlEvent]) -> Result<(), Error> {
        let mut next = self.clone();
        next.observations
            .apply(
                events
                    .iter()
                    .filter(|e| e.kind() == "observation")
                    .map(|e| e.payload().as_bytes()),
            )
            .map_err(Error::Observation)?;
        next.audits
            .apply(
                events
                    .iter()
                    .filter(|e| e.kind() == "trigger_event")
                    .map(|e| e.payload().as_bytes()),
            )
            .map_err(Error::Audit)?;
        if events
            .iter()
            .any(|e| matches!(e.kind(), "observation" | "trigger_event"))
        {
            next.display_identity = Arc::new(());
        }
        // The control owner handles session_state; only source executes input_event.
        // Other messages retain the mainline viewer's side-effect-free policy, with reliable-cursor advancement still owned by outer layers.
        *self = next;
        Ok(())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_relay_viewer_business_tests.rs"]
mod tests;
