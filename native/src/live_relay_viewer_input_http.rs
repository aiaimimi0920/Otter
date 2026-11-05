//! Surface观看输入合同；同一串行租约拥有actor双游标，不把HTTP接受当成源已执行。
use super::*;
use crate::live_input::{Button, Event, MAX_SEQUENCE};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Default)]
pub(crate) struct Capabilities(u16);
impl Capabilities {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, Error> {
        #[derive(Deserialize)]
        struct Root {
            session: Fields,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fields {
            #[serde(default)]
            interaction_capabilities: Vec<String>,
        }
        let root: Root = protocol::parse(bytes).map_err(Error::Protocol)?;
        let mut bits = 0;
        for name in root.session.interaction_capabilities {
            let bit = match name.as_str() {
                "pointer_move" => 1,
                "pointer_button" => 2,
                "double_click" => 4,
                "drag" => 8,
                "wheel" => 16,
                "keyboard" => 32,
                "cancel" => 64,
                "text" => 128,
                "focus" => 256,
                _ => return Err(Error::Input),
            };
            if bits & bit != 0 {
                return Err(Error::Input);
            }
            bits |= bit;
        }
        Ok(Self(bits))
    }
    pub(crate) fn available(self) -> bool {
        self.0 & 64 != 0 && self.0 & 51 != 0
    }
    pub(crate) fn allows(self, event: Event) -> bool {
        let required = match event {
            Event::Move { .. } => 1,
            Event::Button { double: true, .. } => 2 | 4,
            Event::Button { .. } => 2,
            Event::Wheel { .. } => 16,
            Event::Key { .. } => 32,
        };
        event.valid() && self.0 & required == required
    }
}
pub(crate) struct Binding<'a> {
    pub surface: &'a str,
    pub attachment: &'a str,
}
fn valid_binding(binding: &Binding<'_>, sequence: u64) -> Result<(), Error> {
    if !protocol::identifier(binding.surface)
        || !protocol::identifier(binding.attachment)
        || sequence == 0
        || sequence > MAX_SEQUENCE
    {
        return Err(Error::Input);
    }
    Ok(())
}
pub(crate) fn event_payload(event: Event) -> Result<Value, Error> {
    if !event.valid() {
        return Err(Error::Input);
    }
    let (kind, data) = match event {
        Event::Move { point: [x, y] } => ("mouse_move", json!({"x":x,"y":y})),
        Event::Button {
            point: [x, y],
            button,
            down,
            double,
        } => (
            "mouse_button",
            json!({
            "x":x,"y":y,"button":match button {Button::Left=>"left",Button::Middle=>"middle",Button::Right=>"right"},
            "state":if down {"pressed"}else{"released"},"clickCount":if double {2}else{1}}),
        ),
        Event::Wheel {
            point: [x, y],
            delta,
            horizontal,
        } => (
            "wheel",
            json!({"x":x,"y":y,
            "deltaX":if horizontal {delta}else{0},"deltaY":if horizontal {0}else{delta}}),
        ),
        Event::Key { code, down } => (
            "key",
            json!({"code":format!("vk:{code}"),
            "state":if down {"pressed"}else{"released"},"modifiers":0}),
        ),
    };
    Ok(json!({"kind":kind,"data":data}))
}
impl Control<'_> {
    pub(crate) fn change_viewer_controller(
        &self,
        identity: &Identity,
        binding: Binding<'_>,
        sequence: u64,
        acquire: bool,
    ) -> Result<(), Error> {
        valid_binding(&binding, sequence)?;
        let bytes = self.send(
            "POST",
            &path(identity.session(), Some("control"))?,
            &serde_json::to_vec(
                &json!({"protocolVersion":protocol::PROTOCOL,"surfaceInstanceId":binding.surface,
                "attachmentId":binding.attachment,"epoch":identity.epoch(),"sequence":sequence,
                "action":if acquire {"acquire"}else{"release"},"leaseDurationMs":30_000}),
            )
            .map_err(|_| Error::Input)?,
            Duration::from_secs(3),
        )?;
        self.validate_controller_reply(identity, acquire, &bytes)
    }
    pub(super) fn validate_controller_reply(
        &self,
        identity: &Identity,
        acquire: bool,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let snapshot = Snapshot::parse(bytes, identity.session()).map_err(Error::Protocol)?;
        snapshot
            .renewal(identity, self.session.device_id())
            .map_err(Error::Protocol)?;
        #[derive(Deserialize)]
        struct Root {
            session: Fields,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fields {
            controller_device: Option<String>,
        }
        let actual: Root = protocol::parse(bytes).map_err(Error::Protocol)?;
        if actual.session.controller_device.as_deref() != acquire.then(|| self.session.device_id())
        {
            return Err(Error::Input);
        }
        Ok(())
    }
    pub(crate) fn forward_window_input(
        &self,
        identity: &Identity,
        binding: Binding<'_>,
        cursor: RequesterCursor,
        event: Event,
    ) -> Result<u64, Error> {
        valid_binding(&binding, cursor.control)?;
        if cursor.input == 0 || cursor.input > MAX_SEQUENCE {
            return Err(Error::Input);
        }
        let body = json!({"surfaceInstanceId":binding.surface,"attachmentId":binding.attachment,
            "envelope":self.window_input_envelope(identity, cursor, event)?});
        let bytes = self.send(
            "POST",
            &path(identity.session(), Some("input"))?,
            &serde_json::to_vec(&body).map_err(|_| Error::Input)?,
            Duration::from_secs(3),
        )?;
        Self::validate_input_reply(&bytes)
    }
    pub(super) fn window_input_envelope(
        &self,
        identity: &Identity,
        cursor: RequesterCursor,
        event: Event,
    ) -> Result<Value, Error> {
        if cursor.control == 0
            || cursor.control > MAX_SEQUENCE
            || cursor.input == 0
            || cursor.input > MAX_SEQUENCE
        {
            return Err(Error::Cursor);
        }
        Ok(
            json!({"protocolVersion":protocol::PROTOCOL,"sessionId":identity.session(),"epoch":identity.epoch(),
            "sequence":cursor.control,"messageType":"input_event","payload":{"inputSequence":cursor.input,
            "issuedAtMs":now()?,"sourceDeviceId":self.session.device_id(),"kind":event_payload(event)?}}),
        )
    }
    pub(super) fn validate_input_reply(bytes: &[u8]) -> Result<u64, Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Ack {
            protocol_version: String,
            accepted_sequence: u64,
        }
        let ack: Ack = protocol::parse(bytes).map_err(Error::Protocol)?;
        if ack.protocol_version != protocol::PROTOCOL
            || ack.accepted_sequence == 0
            || ack.accepted_sequence > MAX_SEQUENCE
        {
            return Err(Error::Cursor);
        }
        Ok(ack.accepted_sequence)
    }
}
