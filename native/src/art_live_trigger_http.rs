//! Device/nonce原通道上的精确本地自动触发操作；不会重放任何未知POST。
use super::*;
use crate::{
    art_live_trigger::{self as trigger, Reply, Scope},
    live_relay_trigger_registration::Condition,
};
use serde_json::{json, Value};
impl LiveControl<'_> {
    pub(crate) fn local_art_trigger(
        &self,
        identity: &Identity,
        proof: &Proof,
        scope: &Scope,
        condition: &Condition,
        operation: Value,
        reducing: bool,
    ) -> Result<Reply, LiveError> {
        if !scope.validate()
            || condition.validate().is_err()
            || scope.session_id != identity.session()
            || scope.epoch != identity.epoch()
            || proof.binding.origin != self.client.origin()
            || proof.binding.device_id != self.session.device_id()
        {
            return Err(LiveError::Input);
        }
        if !reducing {
            proof.validate()?;
        }
        let body = serde_json::to_vec(
            &json!({"protocolVersion":trigger::PROTOCOL,"binding":proof.binding,
            "grant":proof.envelope,"operation":operation}),
        )
        .map_err(|_| LiveError::Input)?;
        if body.len() > 32 * 1024 {
            return Err(LiveError::Input);
        }
        let bytes = self.send(
            "POST",
            "/v1/art/local/live-trigger",
            &body,
            Duration::from_secs(3),
        )?;
        if !reducing {
            proof.validate()?;
        }
        Reply::parse(
            &bytes,
            &proof.binding,
            proof.grant_id(),
            scope,
            condition,
            identity,
        )
    }
}
