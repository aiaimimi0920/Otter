//! One-shot decisions from explicit user choices; own the original prompt and prohibit replay or reversing the choice after a network attempt.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Approve,
    Reject,
}
pub struct PrepareError {
    pub request: Request,
    pub error: Error,
}
pub struct Decision {
    request: Request,
    choice: Choice,
    bytes: Vec<u8>,
    attempted: bool,
    confirmed: bool,
}
impl Decision {
    /// Call only after an actual user choice; there is no default choice, and approval must not be inferred from the payload.
    pub fn new(
        request: Request,
        event: &ClickEvent,
        ack: &Ack,
        choice: Choice,
        now_ms: u64,
    ) -> Result<Self, Box<PrepareError>> {
        if let Err(error) = request.validate(event, ack, now_ms) {
            return Err(Box::new(PrepareError { request, error }));
        }
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            protocol_version: &'a str,
            confirmation_id: &'a str,
            instance_id: &'a str,
            attachment_id: &'a str,
            device_id: &'a str,
            approved: bool,
        }
        let wire = Wire {
            protocol_version: "loom.surface.v1",
            confirmation_id: request.id(),
            instance_id: &request.wire.instance_id,
            attachment_id: &request.wire.attachment_id,
            device_id: &request.wire.device_id,
            approved: choice == Choice::Approve,
        };
        let bytes = match serde_json::to_vec(&wire) {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err(Box::new(PrepareError {
                    request,
                    error: Error::Protocol,
                }))
            }
        };
        Ok(Self {
            request,
            choice,
            bytes,
            attempted: false,
            confirmed: false,
        })
    }
    pub fn choice(&self) -> Choice {
        self.choice
    }
    pub fn attempted(&self) -> bool {
        self.attempted
    }
    /// Indicates only successful decision HTTP submission, not action execution or a scene update.
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn begin_submission(&mut self) {
        self.attempted = true;
    }
    pub fn acknowledge(&mut self) {
        self.confirmed = true;
    }
    pub fn matches(&self, event: &ClickEvent, ack: &Ack) -> bool {
        self.request.binding == *event.binding()
            && self.request.wire.event_id == event.id()
            && self.request.wire.request_id == ack.request_id()
            && ack.matches_event(event)
    }
}
