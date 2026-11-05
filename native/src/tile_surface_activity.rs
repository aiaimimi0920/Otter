//! Wall view的有界操作状态；只接受当前attachment的身份，不将ACK当正式结果。
use crate::{
    surface_action_ack::{Ack, Status},
    surface_confirmation::Risk,
    tile_surface_wire::{id, integer, View},
    tile_wire::Result,
};
use serde::Deserialize;
use serde_json::Value;
#[derive(Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Confirmation {
    pub protocol_version: String,
    pub confirmation_id: String,
    pub instance_id: String,
    pub attachment_id: String,
    pub device_id: String,
    pub hook_node_id: String,
    pub event_id: String,
    pub request_id: String,
    pub action_id: String,
    pub risk: Risk,
    pub expires_at_ms: u64,
    pub payload: Option<Value>,
}
#[cfg(test)]
#[path = "tile_surface_activity_tests.rs"]
mod tests;
pub(crate) struct Pending {
    pub ack: Ack,
    pub event: String,
    pub action: String,
    pub cancelable: bool,
}
#[derive(Default)]
pub(crate) struct Activity {
    pub confirmations: Vec<Confirmation>,
    pub pending: Vec<Pending>,
    pub failure: Option<String>,
}
impl Activity {
    pub fn parse(v: &Value, view: &View) -> Result<Self> {
        let confirmations = v["confirmations"]
            .as_array()
            .ok_or("tile_surface_confirmations")?;
        let pending = v["pending"].as_array().ok_or("tile_surface_pending")?;
        if confirmations.len() > 64 || pending.len() > 64 {
            return Err("tile_surface_activity_limit");
        }
        let mut result = Self::default();
        let mut ids = std::collections::BTreeSet::new();
        for value in confirmations {
            if serde_json::to_vec(value)
                .map_err(|_| "tile_surface_confirmation")?
                .len()
                > crate::surface_confirmation::MAX_BYTES
            {
                return Err("tile_surface_confirmation_limit");
            }
            let c: Confirmation =
                serde_json::from_value(value.clone()).map_err(|_| "tile_surface_confirmation")?;
            if c.protocol_version != "loom.surface.v1"
                || c.instance_id != view.instance_id
                || c.attachment_id != view.attachment_id
                || c.hook_node_id != format!("wall:{}", view.binding.endpoint_id)
                || c.expires_at_ms == 0
                || c.expires_at_ms > crate::tile_wire::MAX_SAFE
                || [
                    &c.confirmation_id,
                    &c.device_id,
                    &c.event_id,
                    &c.request_id,
                    &c.action_id,
                ]
                .iter()
                .any(|s| !crate::tile_wire::identifier(s))
                || !ids.insert(c.confirmation_id.clone())
            {
                return Err("tile_surface_confirmation_identity");
            }
            result.confirmations.push(c);
        }
        ids.clear();
        for value in pending {
            let event = id(&value["ack"]["eventId"])?;
            let bytes = serde_json::to_vec(&value["ack"]).map_err(|_| "tile_surface_ack")?;
            let ack = Ack::parse_identity(&bytes, &view.instance_id, event)
                .map_err(|_| "tile_surface_ack")?;
            if !ack.accepted()
                || !matches!(
                    ack.status(),
                    Status::Accepted | Status::Queued | Status::Running | Status::CancelRequested
                )
                || !ids.insert(ack.request_id().to_owned())
            {
                return Err("tile_surface_pending_identity");
            }
            result.pending.push(Pending {
                ack,
                event: event.into(),
                action: id(&value["actionId"])?.into(),
                cancelable: value["cancelable"]
                    .as_bool()
                    .ok_or("tile_surface_cancelable")?,
            });
        }
        if !v["failure"].is_null() {
            if v["failure"]["code"] != "wall_surface_action_failed" {
                return Err("tile_surface_failure");
            }
            result.failure = Some(id(&v["failure"]["requestId"])?.into());
        }
        integer(&v["generation"])?;
        Ok(result)
    }
}
