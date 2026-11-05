//! Surface 动作 ACK 的有界身份校验；接收不等于成功，成功也不等于场景已收敛。
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
    /// 不可信远端文本；呈现端须按纯文本处理，不可拼入命令或未经脱敏写日志。
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
    /// initial ACK 不带 attachment/generation；只核对协议实际提供的身份，不臆造保证。
    pub fn parse(bytes: &[u8], event: &ClickEvent) -> Result<Self, Error> {
        Self::parse_identity(bytes, event.binding().instance(), event.id())
    }
    /// 纯ACK身份校验；Wall owner另行核view/generation/sequence，不伪造普通Binding。
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
    /// 后续 ACK 必须仍属于同一 request；不允许另一次请求接管原动作。
    pub fn update(&mut self, bytes: &[u8], event: &ClickEvent) -> Result<(), Error> {
        let next = Self::parse(bytes, event)?;
        if next.0.instance_id != self.0.instance_id
            || next.0.event_id != self.0.event_id
            || next.0.request_id != self.0.request_id
        {
            return Err(Error::Identity);
        }
        *self = next;
        Ok(())
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
