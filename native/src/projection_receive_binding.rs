//! 接收意图与本地 Unit 的持久关联；本地元数据不是远端授权或 displayed 证明。
use super::{accept::Verified, envelope, journal::Ticket, receive_route, Envelope, Error};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    role: String,
    local_unit_id: String,
    envelope: Envelope,
    revision: u64,
    digest: String,
    linked: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    stopped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stop_reason: Option<StopReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_pixels: Option<source::Proof>,
    otter_native_receive: Intent,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "receive_route::some"
    )]
    offline_transport: Option<receive_route::Transport>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Intent {
    intent_id: String,
    device_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "receive_route::some"
    )]
    peer_id: Option<String>,
}
impl Binding {
    pub(crate) fn stopped(&self) -> bool {
        self.stopped
    }
    pub(crate) fn stop(&self, reason: StopReason) -> Self {
        let mut next = self.clone();
        next.stopped = true;
        next.stop_reason = Some(reason);
        next
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
    pub(crate) fn unit(&self) -> &str {
        &self.local_unit_id
    }
    pub(crate) fn new(value: &Verified, ticket: &Ticket) -> Result<Self, Error> {
        Self::new_cancellable(value, ticket, || false)
    }
    pub(crate) fn new_cancellable(
        value: &Verified,
        ticket: &Ticket,
        cancelled: impl FnMut() -> bool,
    ) -> Result<Self, Error> {
        let binding = Self {
            role: "receiver".into(),
            local_unit_id: value.unit().into(),
            envelope: value.envelope().clone(),
            revision: value.revision(),
            digest: value.digest().into(),
            linked: true,
            stopped: false,
            stop_reason: None,
            source_pixels: Some(
                source::Proof::capture(value.image(), cancelled).ok_or(Error::Snapshot)?,
            ),
            otter_native_receive: Intent {
                intent_id: ticket.intent_id().into(),
                device_id: ticket.device().into(),
                peer_id: ticket.invitation().peer_id().map(str::to_owned),
            },
            offline_transport: ticket
                .invitation()
                .peer_id()
                .map(|_| receive_route::Transport {
                    origin: ticket.origin().into(),
                }),
        };
        if value.intent_id.as_deref() != Some(ticket.intent_id())
            || value.device() != ticket.device()
            || !binding.matches_ticket(ticket)
        {
            return Err(Error::Response);
        }
        Ok(binding)
    }
    pub(crate) fn valid(&self, unit: &str) -> bool {
        self.envelope.validate(self.envelope.origin()).is_ok()
            && self.role == "receiver"
            && self.linked
            && self.local_unit_id == unit
            && unit == format!("delivery-{}", &self.envelope.projection_id()[11..])
            && (self.envelope.source_revision()..=envelope::MAX_REVISION).contains(&self.revision)
            && envelope::digest(&self.digest)
            && envelope::identifier(&self.otter_native_receive.device_id)
            && super::journal::record_intent_id(&self.otter_native_receive.intent_id)
            && match (&self.offline_transport, &self.otter_native_receive.peer_id) {
                (None, None) => true,
                (Some(route), Some(peer)) => {
                    receive_route::Offline::new(&route.origin, peer).is_ok()
                }
                _ => false,
            }
    }
    pub(crate) fn matches_ticket(&self, ticket: &Ticket) -> bool {
        self.valid(ticket.unit())
            && &self.envelope == ticket.envelope()
            && self.otter_native_receive.device_id == ticket.device()
            && self.otter_native_receive.intent_id == ticket.intent_id()
            && self.revision >= ticket.invitation().revision()
            && self
                .offline_transport
                .as_ref()
                .map(|route| route.origin.as_str())
                == ticket.invitation().peer_id().map(|_| ticket.origin())
            && self.otter_native_receive.peer_id.as_deref() == ticket.invitation().peer_id()
    }
}
#[path = "projection_receive_checkpoint.rs"]
pub mod checkpoint;
#[path = "projection_binding_source.rs"]
mod source;

/// 只保合同中的终态短码；不存服务端任意错误正文或凭证。
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    ProjectionUnlinked,
    ProjectionInvitationExpired,
    ProjectionSourceRevoked,
    ProjectionAccessDenied,
    ProjectionNotFound,
    ProjectionSourceMismatch,
    ProjectionRejected,
    ProjectionPeerRevoked,
}

#[cfg(test)]
#[path = "projection_receive_binding_tests.rs"]
mod tests;
