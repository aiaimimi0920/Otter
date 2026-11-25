//! Bounded identity validation for Surface action ACKs; receipt is not success, and success is not scene convergence.
use crate::surface_click_event::ClickEvent;
use serde::Deserialize;
pub const MAX_ACK_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Accepted,
    AwaitingConfirmation,
    Queued,
    Running,
    CancelRequested,
    Cancelled,
    Succeeded,
    Failed,
    Interrupted,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Json,
    Identity,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Failure {
    code: String,
    message: String,
    detail: Option<String>,
}
impl Failure {
    pub fn code(&self) -> &str {
        &self.code
    }
    /// Untrusted remote text; presenters must treat it as plain text, never interpolate it into commands or log it unsanitized.
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    protocol_version: String,
    instance_id: String,
    event_id: String,
    request_id: String,
    accepted: bool,
    status: Status,
    error: Option<Failure>,
}
#[derive(Clone, Debug)]
pub struct Ack(Wire);
impl Ack {
    pub(crate) fn matches_event(&self, event: &ClickEvent) -> bool {
        self.0.instance_id == event.binding().instance() && self.0.event_id == event.id()
    }
    /// The initial ACK has no attachment/generation; verify only identities actually supplied by the protocol without inventing guarantees.
    pub fn parse(bytes: &[u8], event: &ClickEvent) -> Result<Self, Error> {
        Self::parse_identity(bytes, event.binding().instance(), event.id())
    }
    /// Pure ACK identity validation; the Wall owner separately checks view/generation/sequence without fabricating an ordinary Binding.
    pub(crate) fn parse_identity(bytes: &[u8], instance: &str, event: &str) -> Result<Self, Error> {
        if bytes.is_empty() || bytes.len() > MAX_ACK_BYTES {
            return Err(Error::Limit);
        }
        let ack: Wire = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        if ack.protocol_version != "loom.surface.v1"
            || ack.instance_id != instance
            || ack.event_id != event
            || !crate::live_relay_viewer_identity::identifier(&ack.request_id)
        {
            return Err(Error::Identity);
        }
        Ok(Self(ack))
    }
    /// Subsequent ACKs must still belong to the same request; another request cannot take over the original action.
    pub fn update(&mut self, bytes: &[u8], event: &ClickEvent) -> Result<(), Error> {
        self.replace_checked(bytes, event).map(drop)
    }
    /// Move out old evidence for stream-convergence rollback; failed validation leaves the current ACK intact.
    pub(crate) fn replace_checked(
        &mut self,
        bytes: &[u8],
        event: &ClickEvent,
    ) -> Result<Self, Error> {
        let next = Self::parse(bytes, event)?;
        if next.0.instance_id != self.0.instance_id
            || next.0.event_id != self.0.event_id
            || next.0.request_id != self.0.request_id
        {
            return Err(Error::Identity);
        }
        Ok(std::mem::replace(self, next))
    }
    pub fn request_id(&self) -> &str {
        &self.0.request_id
    }
    pub fn accepted(&self) -> bool {
        self.0.accepted
    }
    pub fn status(&self) -> Status {
        self.0.status
    }
    pub fn failure(&self) -> Option<&Failure> {
        self.0.error.as_ref()
    }
    pub fn succeeded(&self) -> bool {
        self.0.accepted && self.0.status == Status::Succeeded
    }
}
