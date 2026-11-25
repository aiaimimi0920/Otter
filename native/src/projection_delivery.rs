//! Traditional delivery contract; publish after complete validation without copying credentials, and receipt does not mean persistence/presentation.
use serde::{Deserialize, Serialize};
#[path = "projection_envelope.rs"]
mod envelope;
pub use envelope::Envelope;
#[cfg(windows)]
#[path = "projection_accept.rs"]
pub mod accept;
#[cfg(windows)]
#[path = "projection_receive_binding.rs"]
pub mod binding;
#[path = "projection_checkpoint_session.rs"]
pub(crate) mod checkpoint_session;
#[cfg(windows)]
#[path = "projection_receive_journal.rs"]
pub mod journal;
#[path = "projection_receive_route.rs"]
mod receive_route;
#[path = "projection_sender.rs"]
pub mod sender;
#[cfg(windows)]
#[path = "projection_share.rs"]
pub(crate) mod share;
#[path = "projection_targets.rs"]
pub mod targets;
pub(crate) use receive_route::Operation as ReceiveOperation;
#[path = "projection_inbox_decode.rs"]
mod inbox_decode;
pub const MAX_JSON: usize = 1024 * 1024;
pub const MAX_INVITATIONS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Json,
    Origin,
    Envelope,
    Entry,
    Duplicate,
    AlreadyAccepted,
    Receipt,
    Response,
    Snapshot,
    Budget,
    Stopped,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InboxPolicy {
    Disabled,
    Confirm,
    Auto,
}
#[derive(Clone, Debug)]
pub struct Invitation {
    envelope: Envelope,
    revision: u64,
    digest: String,
    source_name: String,
    accepted: bool,
    offline: Option<receive_route::Offline>,
    shared_origin: Option<String>,
    public_invitation: bool,
}
impl Invitation {
    pub(crate) fn requires_receipt(&self) -> bool {
        !self.public_invitation
    }
    pub fn envelope(&self) -> &Envelope {
        &self.envelope
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
    pub fn accepted(&self) -> bool {
        self.accepted
    }
    /// Reject only new invitations awaiting local consent; new receive policies must not affect existing accepted associations.
    pub(crate) fn reject_body(&self, origin: &str) -> Result<Vec<u8>, Error> {
        self.validate_for(origin)?;
        if self.accepted {
            return Err(Error::AlreadyAccepted);
        }
        // Rejection grants no receive rights; expired invitations may still be recorded/cleaned by servers, without accept deadlines.
        serde_json::to_vec(&serde_json::json!({
            "projectionId":self.envelope.projection_id(), "status":"rejected"
        }))
        .map_err(|_| Error::Json)
    }
}
#[derive(Debug)]
pub struct Inbox {
    entries: Vec<Invitation>,
    offline_peers: bool,
}
impl Inbox {
    pub fn entries(&self) -> &[Invitation] {
        &self.entries
    }
    /// Keep markers while offline queries are incomplete and clear only after successful merge; shared results are not complete cross-Loom inboxes.
    pub fn has_offline_peers(&self) -> bool {
        self.offline_peers
    }
    pub fn into_entries(self) -> Vec<Invitation> {
        self.entries
    }
}
pub(crate) fn receipt(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAX_JSON {
        return Err(Error::Limit);
    }
    #[derive(Deserialize)]
    struct Receipt {
        recorded: bool,
    }
    let receipt: Receipt = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
    if receipt.recorded {
        Ok(())
    } else {
        Err(Error::Receipt)
    }
}
/// Unlink only reduces privileges; still bind original HTTPS origin strictly and let protocol owners uniformly validate envelopes.
pub(crate) fn unlink_body(envelope: &Envelope, origin: &str) -> Result<Vec<u8>, Error> {
    envelope.validate(origin)?;
    serde_json::to_vec(&serde_json::json!({"projectionId": envelope.projection_id()}))
        .map_err(|_| Error::Json)
}
#[cfg(windows)]
pub(crate) fn displayed_body(
    intent: &journal::Ticket,
    origin: &str,
    device: &str,
) -> Result<Vec<u8>, Error> {
    intent.invitation().validate_for(origin)?;
    if intent.device() != device {
        return Err(Error::Response);
    }
    serde_json::to_vec(&serde_json::json!({
        "projectionId":intent.envelope().projection_id(), "status":"displayed"
    }))
    .map_err(|_| Error::Json)
}
#[cfg(test)]
#[path = "projection_delivery_tests.rs"]
pub(crate) mod tests;
