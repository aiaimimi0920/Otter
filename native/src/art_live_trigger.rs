//! 本地自动触发的严格wire；宿主授权不交给JS，条件仍只由Loom求值。
use crate::{
    art_execution_grant::wire::Binding, live_relay_observation::Observation,
    live_relay_trigger_registration::Condition, live_relay_viewer_identity::Identity,
};
use serde::{Deserialize, Serialize};
pub(crate) const PROTOCOL: &str = "loom.art.local.live-trigger.v1";
pub(crate) const MAX_DELIVERY: usize = 16 * 1024;
pub(crate) const MAX_SEQUENCE: u64 = crate::live_input::MAX_SEQUENCE;
#[path = "art_live_trigger_queue.rs"]
pub(crate) mod queue;
#[path = "art_live_trigger_ticket.rs"]
pub(crate) mod ticket;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Scope {
    pub session_id: String,
    pub epoch: u64,
    pub binding_id: String,
    pub authorization_id: String,
}
pub(crate) fn identifier(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 160
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
}
impl Scope {
    pub fn validate(&self) -> bool {
        [&self.session_id, &self.binding_id, &self.authorization_id]
            .into_iter()
            .all(|v| identifier(v))
            && self.binding_id.starts_with("local-art:")
            && (1..=MAX_SEQUENCE).contains(&self.epoch)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Receipt {
    Applied { formal_revision: u64 },
    NoResult,
    Failed,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    Idle,
    Ready,
    Claimed,
    Acknowledged,
    Expired,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StopReason {
    UserStopped,
    GrantExpired,
    ClockRegressed,
    DeliveryExpired,
    ViewerDisconnected,
    ApprovalChanged,
    DeliveryLimit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Delivery {
    pub delivery_id: String,
    pub callback: String,
    pub condition_revision: u64,
    pub observation: Observation,
    pub source_device_id: String,
    pub expires_at_ms: u64,
}
impl Delivery {
    pub fn validate(&self) -> bool {
        identifier(&self.delivery_id)
            && identifier(&self.source_device_id)
            && self.callback == "onLiveTrigger"
            && (1..=MAX_SEQUENCE).contains(&self.condition_revision)
            && (1..=MAX_SEQUENCE).contains(&self.observation.sequence)
            && self.expires_at_ms <= MAX_SEQUENCE
            && self.expires_at_ms > self.observation.observed_at_ms
            && self.expires_at_ms <= self.observation.observed_at_ms.saturating_add(30_000)
            && self.observation.validate().is_ok()
            && serde_json::to_vec(self).is_ok_and(|b| b.len() <= MAX_DELIVERY)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Reply {
    pub protocol_version: String,
    pub binding: Binding,
    pub grant_id: String,
    pub scope: Scope,
    pub condition: Condition,
    pub enabled: bool,
    pub status: Status,
    pub reason: Option<StopReason>,
    pub delivery_id: Option<String>,
    pub receipt: Option<Receipt>,
    pub delivery: Option<Delivery>,
}
impl Reply {
    pub fn parse(
        bytes: &[u8],
        binding: &Binding,
        grant: &str,
        scope: &Scope,
        condition: &Condition,
        identity: &Identity,
    ) -> Result<Self, crate::live_relay_http::Error> {
        use crate::live_relay_http::Error;
        if bytes.is_empty() || bytes.len() > 32 * 1024 {
            return Err(Error::Input);
        }
        let r: Self = serde_json::from_slice(bytes).map_err(|_| Error::Input)?;
        if r.protocol_version != PROTOCOL
            || &r.binding != binding
            || r.grant_id != grant
            || &r.scope != scope
            || !r.scope.validate()
            || &r.condition != condition
            || r.condition.validate().is_err()
            || r.condition.revision > MAX_SEQUENCE
            || r.scope.session_id != identity.session()
            || r.scope.epoch != identity.epoch()
            || r.delivery_id.as_ref().is_some_and(|id| !identifier(id))
        {
            return Err(Error::Input);
        }
        let valid = match r.status {
            Status::Idle => {
                r.enabled && r.reason.is_none() && r.delivery_id.is_none() && r.receipt.is_none()
            }
            Status::Ready | Status::Claimed => {
                r.enabled && r.reason.is_none() && r.delivery_id.is_some() && r.receipt.is_none()
            }
            Status::Acknowledged => {
                r.enabled && r.reason.is_none() && r.delivery_id.is_some() && r.receipt.is_some()
            }
            Status::Expired | Status::Stopped => !r.enabled && r.reason.is_some(),
        };
        if !valid
            || matches!(r.receipt, Some(Receipt::Applied { formal_revision }) if !(1..=MAX_SEQUENCE).contains(&formal_revision))
            || r.delivery.as_ref().is_some_and(|d| {
                r.status != Status::Claimed
                    || !d.validate()
                    || Some(&d.delivery_id) != r.delivery_id.as_ref()
                    || d.condition_revision != condition.revision
                    || d.observation.observation_id != condition.observation_id
                    || d.source_device_id != identity.source_device()
            })
        {
            return Err(Error::Input);
        }
        Ok(r)
    }
}
/// 只向child交无凭据事件，不携带签名grant或可写宿主身份。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Event {
    pub session_id: String,
    pub epoch: u64,
    pub delivery: Delivery,
}
impl Event {
    pub fn validate(&self) -> bool {
        identifier(&self.session_id)
            && (1..=MAX_SEQUENCE).contains(&self.epoch)
            && self.delivery.validate()
    }
}

#[cfg(test)]
#[path = "art_live_trigger_tests.rs"]
pub(crate) mod tests;
