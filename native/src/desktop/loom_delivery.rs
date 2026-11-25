//! One official-delivery owner: record remote leases before decoding so local-result failures preserve release responsibility.
use super::loom_output_batch::{self, Batch};
use crate::{
    graph_requests::Request,
    loom_art_control::{self, Outcome},
    loom_output_leases::{self, Leases},
    workspace_id::UnitId,
};
use serde_json::Value;
use std::sync::atomic::AtomicBool;

pub struct Delivery {
    unit: UnitId,
    request: Request,
    result: Result<Batch, loom_output_batch::Error>,
    leases: Leases,
}
#[derive(Debug, PartialEq)]
pub enum ReleaseError {
    IdentityGeneration,
    Lease(loom_output_leases::Error),
    Transport(loom_art_control::Error),
    Rejected(Outcome),
}
#[derive(Debug, PartialEq)]
pub enum TakeError {
    PendingRelease,
    Decode(loom_output_batch::Error),
}
impl Delivery {
    /// Transfer local results once only; unconfirmed leases/results remain entirely with this owner and failures consume neither.
    /// Even decoding errors transfer only after release completes; this method performs no networking/image copying.
    pub fn take_released(&mut self) -> Result<Batch, TakeError> {
        if !self.leases.is_empty() {
            return Err(TakeError::PendingRelease);
        }
        std::mem::replace(&mut self.result, Err(loom_output_batch::Error::Consumed))
            .map_err(TakeError::Decode)
    }
    /// Authenticated remote-code delivery retains actual identity; install local derived batches only after original results are collected once and leases cleared.
    /// Create no Delivery/lease and do not alter remote-release state. Callers must finish all background preparation before installation.
    pub(super) fn install_derived(&mut self, batch: Batch) -> Result<(), TakeError> {
        if !self.leases.is_empty() {
            return Err(TakeError::PendingRelease);
        }
        if !matches!(self.result, Err(loom_output_batch::Error::Consumed)) {
            return Err(TakeError::Decode(loom_output_batch::Error::Consumed));
        }
        self.result = Ok(batch);
        Ok(())
    }
    /// Use discovery identity fixed at execution time, not rediscovering Loom instances that may have restarted/changed.
    /// Background only: TLS connections take at most three seconds and control exchanges five; failures allow explicit retries.
    pub fn release_authenticated(
        &mut self,
        identity: &crate::loom_bridge_identity::Identity,
    ) -> Result<(), ReleaseError> {
        if self.leases.is_empty() {
            return Ok(());
        }
        let id =
            super::document_state::new_unit_id().map_err(|_| ReleaseError::IdentityGeneration)?;
        let release_id =
            UnitId::parse(&format!("release:{id}")).ok_or(ReleaseError::IdentityGeneration)?;
        self.release_with(release_id, |message, request_id| {
            let socket =
                crate::loom_bridge_client::connect(identity, std::time::Duration::from_secs(5))
                    .map_err(|_| loom_art_control::Error::Closed)?;
            loom_art_control::run(socket, message, request_id)
        })
    }
    /// Callers retain original responses. Do not decode invalid leases; callers must handle unsupported responses.
    pub fn prepare(
        unit: UnitId,
        request: Request,
        outputs: &Value,
        candidates: Option<&Value>,
        cancel: &AtomicBool,
    ) -> Result<Self, loom_output_leases::Error> {
        let leases = Leases::outputs(unit.clone(), request.clone(), outputs)?;
        let result = loom_output_batch::decode(outputs, candidates, cancel);
        Ok(Self {
            unit,
            request,
            result,
            leases,
        })
    }
    pub fn unit(&self) -> &UnitId {
        &self.unit
    }
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn result(&self) -> Result<&Batch, &loom_output_batch::Error> {
        self.result.as_ref()
    }
    pub fn pending_release_count(&self) -> usize {
        self.leases.pending_count()
    }
    /// Background call using authenticated control transports; failures retain both local results and remote leases.
    /// Execution owners provide new IDs; this method neither retries, spawns threads, nor networks in Drop.
    pub fn release_with(
        &mut self,
        release_id: UnitId,
        transport: impl FnOnce(String, &UnitId) -> Result<Outcome, loom_art_control::Error>,
    ) -> Result<(), ReleaseError> {
        if self.leases.is_empty() {
            return Ok(());
        }
        let message = self
            .leases
            .begin_release(release_id.clone())
            .map_err(ReleaseError::Lease)?;
        match transport(message, &release_id) {
            Ok(outcome) => {
                let success = outcome == Outcome::Succeeded;
                self.leases
                    .complete_release(&release_id, Some(outcome.clone()));
                if success {
                    Ok(())
                } else {
                    Err(ReleaseError::Rejected(outcome))
                }
            }
            Err(error) => {
                self.leases.complete_release(&release_id, None);
                Err(ReleaseError::Transport(error))
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "loom_delivery_tests.rs"]
mod tests;
