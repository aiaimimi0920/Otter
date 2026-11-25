//! Native extension sessions, authorization, and command exchange use local TLS/Hook identity; document owners still review/commit result effects.
use crate::{
    extension_when::Context,
    loom_bridge_client::BridgeSocket,
    loom_bridge_identity::Identity,
    loom_bridge_stream::Interrupt,
    loom_extension_snapshot::{self as catalog, Snapshot},
};
use ring::rand::{SecureRandom, SystemRandom};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
#[path = "loom_extension_invoke.rs"]
mod invoke;
#[path = "loom_extension_resources.rs"]
mod resources;
#[path = "loom_extension_result.rs"]
mod result;
#[cfg(test)]
#[path = "loom_extension_test_peer.rs"]
pub(crate) mod test_peer;
#[cfg(test)]
#[path = "loom_extension_session_tests.rs"]
mod tests;
#[path = "loom_extension_wire.rs"]
mod wire;
pub use resources::{Access, Attachment, Image, Prepared};
pub use result::{Effect, EffectKind, Outcome};

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Timeout,
    Closed,
    Wire,
    Limit,
    Random,
    Unsupported,
    Unavailable,
    AuthorityChanged,
    Denied,
    Remote { code: String, message: String },
    Hook(crate::loom_handshake_exchange::Error),
    Snapshot(catalog::Error),
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    unit_id: String,
    revision: u64,
}
impl Target {
    pub fn new(unit: &str, revision: u64) -> Result<Self, Error> {
        if !catalog::text(unit, 384) || revision > catalog::MAX_INTEGER {
            return Err(Error::Wire);
        }
        Ok(Self {
            unit_id: unit.into(),
            revision,
        })
    }
}
pub struct Session {
    socket: Option<BridgeSocket>,
    id: String,
    snapshot: Option<Arc<Snapshot>>,
    ocr_text: bool,
}
fn request_id() -> Result<String, Error> {
    let mut bytes = [0; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Random)?;
    Ok(format!(
        "extension:{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
impl Session {
    /// Background owners only; no discovery-file reads, thread creation, or arbitrary UI URLs.
    pub fn open(identity: &Identity, cancel: &AtomicBool) -> Result<Self, Error> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let (hook, mut socket) = crate::loom_handshake_exchange::open(identity, cancel).map_err(
            |error| match error {
                crate::loom_handshake_exchange::Error::Cancelled => Error::Cancelled,
                error => Error::Hook(error),
            },
        )?;
        socket.set_config(|config| {
            config.max_message_size = Some(wire::MAX_BYTES);
            config.max_frame_size = Some(wire::MAX_BYTES);
        });
        let id = request_id()?;
        let request = serde_json::json!({"method":"loom.extension.handshake", "params":{
            "requestId":id, "hookSessionId":hook.session_id(), "protocol":catalog::PROTOCOL,
            "apiVersion":catalog::API, "requiredFeatures":["contribution.snapshot", "command.invoke", "resource.authorization.v1"],
            "optionalFeatures":["shortcut.registry", "menu.registry", "notice.effects", "ocr-text.v1"] }});
        let response = wire::exchange(
            &mut socket,
            &mut None,
            &id,
            request,
            Duration::from_secs(10),
            cancel,
        )?;
        let response = wire::handshake(&response)?;
        Ok(Self {
            socket: Some(socket),
            id: response.0,
            snapshot: Some(Arc::new(response.1)),
            ocr_text: response.2,
        })
    }
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.socket.as_ref()?;
        self.snapshot.as_deref()
    }
    pub fn supports_ocr_text(&self) -> bool {
        self.ocr_text
    }
    /// Read-only catalog projection, not resource-permit issuance; session owners must revoke UI references on termination.
    pub fn snapshot_handle(&self) -> Option<Arc<Snapshot>> {
        self.socket.as_ref()?;
        self.snapshot.clone()
    }
    /// Background subscriptions block for actual catalog-generation changes; no polling RPCs, and cancellation wakes through existing Interrupt.
    pub fn wait_update(&mut self, cancel: &AtomicBool) -> Result<(), Error> {
        let result = wire::wait_update(
            self.socket.as_mut().ok_or(Error::Closed)?,
            &mut self.snapshot,
            cancel,
        );
        if result.is_err() {
            self.stop();
        }
        result
    }
    pub fn interrupt(&self) -> Result<Interrupt, Error> {
        self.socket
            .as_ref()
            .ok_or(Error::Closed)?
            .get_ref()
            .sock
            .interrupt_handle()
            .map_err(|_| Error::Closed)
    }
    /// Actual remote checkOnly validation without resource-lease issuance; execution and attachment CAS still require their own transactions.
    pub fn authorize(
        &mut self,
        command: &str,
        context: &Context<'_>,
        target: &Target,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        if cancel.load(Ordering::Acquire) {
            self.stop();
            return Err(Error::Cancelled);
        }
        let snapshot = self.snapshot().ok_or(Error::Closed)?;
        let authority = snapshot
            .command(command, context)
            .ok_or(Error::Unavailable)?;
        let generation = snapshot.generation();
        let id = request_id()?;
        let request = serde_json::json!({"method":"loom.extension.command.authorize", "params":{
            "requestId":id, "sessionId":self.id, "pluginId":authority.plugin(), "commandId":authority.id(),
            "snapshotGeneration":generation, "target":target, "checkOnly":true }});
        let result = wire::exchange(
            self.socket.as_mut().ok_or(Error::Closed)?,
            &mut self.snapshot,
            &id,
            request,
            Duration::from_secs(5),
            cancel,
        );
        // After catalog-generation changes, even successful responses do not prove old-button permissions remain valid; never replay this request.
        let result = result.and_then(|data| {
            if data["authorized"] != true {
                return Err(Error::Denied);
            }
            Ok(())
        });
        self.settle(generation, result)
    }
    fn settle<T>(&mut self, generation: u64, result: Result<T, Error>) -> Result<T, Error> {
        let result = if self
            .snapshot
            .as_ref()
            .is_some_and(|s| s.generation() != generation)
        {
            Err(Error::AuthorityChanged)
        } else {
            result
        };
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn stop(&mut self) {
        // Do not retain failed sessions' old authority or idle connections; the same owner closes them without background retries.
        self.snapshot = None;
        self.socket = None;
    }
}

fn remaining(
    socket: &mut BridgeSocket,
    deadline: Instant,
    cap: Duration,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(remaining.min(cap))
        .map_err(|_| transport_error(cancel))
}

// Interrupt can take effect between pre-call cancellation checks and actual I/O; error classification must recheck the cancellation owner.
fn transport_error(cancel: &AtomicBool) -> Error {
    if cancel.load(Ordering::Acquire) {
        Error::Cancelled
    } else {
        Error::Closed
    }
}
