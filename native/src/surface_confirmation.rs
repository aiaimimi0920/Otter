//! 确认请求是待展示数据，不是批准能力；使用时必须复核原事件、当前 ACK 和到期时间。
use crate::{
    surface_action_ack::{Ack, Status},
    surface_click_event::ClickEvent,
    surface_live_binding::Binding,
};
use serde::Deserialize;
use serde_json::value::RawValue;
pub const MAX_BYTES: usize = 64 * 1024;
#[path = "surface_confirmation_decision.rs"]
mod decision;
pub use decision::{Choice, Decision, PrepareError};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Protocol,
    Identity,
    State,
    Expired,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Low,
    Medium,
    High,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    protocol_version: String,
    confirmation_id: String,
    instance_id: String,
    attachment_id: String,
    device_id: String,
    hook_node_id: String,
    event_id: String,
    request_id: String,
    action_id: String,
    risk: Risk,
    expires_at_ms: u64,
    payload: Option<Box<RawValue>>,
}
pub struct Request {
    wire: Wire,
    binding: Binding,
}
impl Request {
    pub fn parse_matching(
        raw: &RawValue,
        event: &ClickEvent,
        ack: &Ack,
    ) -> Result<Option<Self>, Error> {
        if raw.get().len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let wire: Wire = serde_json::from_str(raw.get()).map_err(|_| Error::Protocol)?;
        if wire.instance_id != event.binding().instance()
            || wire.event_id != event.id()
            || wire.request_id != ack.request_id()
        {
            return Ok(None);
        }
        #[derive(Deserialize)]
        struct Event<'a> {
            action: &'a str,
        }
        let expected: Event<'_> =
            serde_json::from_slice(event.bytes()).map_err(|_| Error::Protocol)?;
        if !ack.matches_event(event)
            || wire.attachment_id != event.binding().attachment()
            || wire.device_id != event.binding().device()
            || wire.hook_node_id != event.binding().node()
            || wire.action_id != expected.action
        {
            return Err(Error::Identity);
        }
        if wire.protocol_version != "loom.surface.v1"
            || wire.expires_at_ms == 0
            || !crate::live_relay_viewer_identity::identifier(&wire.confirmation_id)
        {
            return Err(Error::Protocol);
        }
        Ok(Some(Self {
            wire,
            binding: event.binding().clone(),
        }))
    }
    pub fn validate(&self, event: &ClickEvent, ack: &Ack, now_ms: u64) -> Result<(), Error> {
        if self.binding != *event.binding()
            || self.wire.event_id != event.id()
            || self.wire.request_id != ack.request_id()
            || !ack.matches_event(event)
        {
            return Err(Error::Identity);
        }
        if !event.attempted() || !ack.accepted() || ack.status() != Status::AwaitingConfirmation {
            return Err(Error::State);
        }
        if now_ms >= self.wire.expires_at_ms {
            return Err(Error::Expired);
        }
        Ok(())
    }
    pub fn id(&self) -> &str {
        &self.wire.confirmation_id
    }
    pub(crate) fn same_prompt(&self, other: &Self) -> bool {
        self.wire.risk == other.wire.risk
            && self.wire.expires_at_ms == other.wire.expires_at_ms
            && self.payload().map(RawValue::get) == other.payload().map(RawValue::get)
    }
    pub fn risk(&self) -> Risk {
        self.wire.risk
    }
    pub fn action(&self) -> &str {
        &self.wire.action_id
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.wire.expires_at_ms
    }
    /// 不可信 JSON，不可直接解释为命令或默认批准理由。
    pub fn payload(&self) -> Option<&RawValue> {
        self.wire.payload.as_deref()
    }
}
