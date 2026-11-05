//! Online verified local-host proof, only consumed by explicit native UI, never exposed to Art JS.
use super::*;
use crate::live_relay_http::{Control as LiveControl, Error as LiveError};
use crate::live_relay_viewer_identity::{Identity, Snapshot};
#[path = "art_live_control_http.rs"]
mod interaction;
#[path = "art_live_trigger_http.rs"]
mod trigger;

pub(crate) const PROTOCOL: &str = "loom.art.local.live-viewer.v1";
pub(crate) struct Proof {
    grant_id: String,
    binding: wire::Binding,
    envelope: wire::Envelope,
    lease: Arc<Lease>,
    permit: Arc<Mutex<Permit>>,
    observed: Mutex<u64>,
}
impl Prepared {
    pub(crate) fn viewer_proof(&self) -> Option<Arc<Proof>> {
        if !self.online()
            || !self
                .request
                .permissions
                .contains(&wire::Permission::Preview)
        {
            return None;
        }
        let proof = Arc::new(Proof {
            grant_id: self.permit.lock().ok()?.grant_id().into(),
            binding: self.request.binding.clone(),
            envelope: serde_json::from_slice(&self.envelope).ok()?,
            lease: self.lease.clone(),
            permit: self.permit.clone(),
            observed: Mutex::new(super::now().ok()?),
        });
        proof.validate().ok()?;
        Some(proof)
    }
}
impl Proof {
    #[cfg(test)]
    pub(crate) fn test_envelope(&self) -> &wire::Envelope {
        &self.envelope
    }
    pub(crate) fn grant_id(&self) -> &str {
        &self.grant_id
    }
    pub(crate) fn binding(&self) -> &wire::Binding {
        &self.binding
    }
    pub(crate) fn validate(&self) -> Result<(), LiveError> {
        // Separate high-water mark: never advance the worker's shared clock from a UI observation.
        let mut observed = self.observed.lock().map_err(|_| LiveError::Credential)?;
        let permit = self.permit.lock().map_err(|_| LiveError::Credential)?;
        let now = super::now()?;
        self.lease
            .validate(&self.binding.origin, now)
            .map_err(|_| LiveError::Credential)?;
        if self.lease.execution_stopped() || now < *observed {
            return Err(LiveError::Credential);
        }
        permit
            .validate_online_host(&self.binding, now)
            .map_err(|_| LiveError::Credential)?;
        *observed = now;
        Ok(())
    }
}
impl LiveControl<'_> {
    pub(crate) fn attach_local_art(
        &self,
        expected: &Identity,
        proof: &Proof,
        sequence: u64,
        require_existing: bool,
    ) -> Result<(Snapshot, crate::live_relay_viewer_business::Business), LiveError> {
        proof.validate()?;
        if proof.binding.origin != self.client.origin()
            || proof.binding.device_id != self.session.device_id()
        {
            return Err(LiveError::Input);
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "protocolVersion":PROTOCOL,"binding":proof.binding,"grant":proof.envelope,
            "envelope":self.viewer_ack(expected, sequence)?,
            "requireExistingMembership":require_existing
        }))
        .map_err(|_| LiveError::Input)?;
        if body.len() > 32 * 1024 {
            return Err(LiveError::Input);
        }
        let bytes = self.send(
            "POST",
            "/v1/art/local/live-viewer/attach",
            &body,
            Duration::from_secs(15),
        )?;
        proof.validate()?;
        self.attached_viewer(expected, &bytes)
    }
}
