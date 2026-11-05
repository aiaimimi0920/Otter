//! Wall动作固定Device POST；一次发送，无未知结果重放，不借普通Surface附件资格。
use super::*;
use crate::{
    surface_action_ack::Ack,
    tile_surface_actions::{Command, Kind},
    tile_surface_wire::State,
};
impl Control<'_> {
    pub(crate) fn wall_surface_command(
        &self,
        command: &Command,
        state: &State,
    ) -> Result<Ack, Error> {
        let a = &command.anchor;
        let (path, body, event) = match &command.kind {
            Kind::Event(event) => (
                "/v1/walls/surfaces/event",
                serde_json::json!({"view":a.view,"placementId":a.placement,"pixel":{"x":a.pixel[0],"y":a.pixel[1]},"sequence":state.sequence.checked_add(1).filter(|n| *n <= wire::MAX_SAFE).ok_or(Error::Input)?,"event":event}),
                event["eventId"].as_str().ok_or(Error::Input)?,
            ),
            Kind::Decision {
                confirmation: c,
                approved,
            } => (
                "/v1/walls/surfaces/confirmation",
                serde_json::json!({"view":a.view,"placementId":a.placement,"pixel":{"x":a.pixel[0],"y":a.pixel[1]},"confirmationId":c.confirmation_id,"approved":approved}),
                c.event_id.as_str(),
            ),
            Kind::Cancel { request, event } => (
                "/v1/walls/surfaces/cancel",
                serde_json::json!({"view":a.view,"requestId":request}),
                event.as_str(),
            ),
        };
        let bytes = self.wall_post(
            path,
            body,
            Duration::from_secs(2),
            crate::surface_action_ack::MAX_ACK_BYTES,
        )?;
        let ack =
            Ack::parse_identity(&bytes, &a.view.instance_id, event).map_err(|_| Error::Input)?;
        let expected = match &command.kind {
            Kind::Event(_) => None,
            Kind::Decision {
                confirmation: c, ..
            } => Some(c.request_id.as_str()),
            Kind::Cancel { request, .. } => Some(request.as_str()),
        };
        if expected.is_some_and(|r| r != ack.request_id()) {
            return Err(Error::Input);
        }
        Ok(ack)
    }
}
