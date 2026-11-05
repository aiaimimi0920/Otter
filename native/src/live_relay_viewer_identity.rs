//! 已认证 HTTP 响应的身份/续期前置校验；解析成功不是认证，不能接受媒体端的快照。
use serde::Deserialize;
pub const PROTOCOL: &str = "loom.live.v1";
pub(super) const MAX_JSON: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Limit,
    Identity,
    Closed,
    Epoch,
    Revision,
    Membership,
    History,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    session: String,
    source_device: String,
    source_hook: String,
    epoch: u64,
}
impl Identity {
    pub fn session(&self) -> &str {
        &self.session
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn source_device(&self) -> &str {
        &self.source_device
    }
    pub fn source_hook(&self) -> &str {
        &self.source_hook
    }
}
pub struct Snapshot {
    identity: Identity,
    revision: u64,
    viewers: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireSnapshot {
    session: WireSession,
    epoch: u64,
    closed: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireSession {
    protocol_version: String,
    session_id: String,
    source_device_id: String,
    source_hook_id: String,
    revision: u64,
    viewer_devices: Vec<String>,
}
pub(super) fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    if bytes.is_empty() || bytes.len() > MAX_JSON {
        return Err(Error::Limit);
    }
    serde_json::from_slice(bytes).map_err(|_| Error::Json)
}
pub(super) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
impl Snapshot {
    /// HTTP transport 必须先验证来源和 Device 凭据，并限制读取量；此处不持有密钥。
    pub fn parse(bytes: &[u8], expected_session: &str) -> Result<Self, Error> {
        let wire: WireSnapshot = parse(bytes)?;
        let s = wire.session;
        if !identifier(expected_session)
            || s.protocol_version != PROTOCOL
            || s.session_id != expected_session
            || !identifier(&s.source_device_id)
            || !identifier(&s.source_hook_id)
        {
            return Err(Error::Identity);
        }
        if wire.closed {
            return Err(Error::Closed);
        }
        if wire.epoch == 0 {
            return Err(Error::Epoch);
        }
        if s.revision == 0 {
            return Err(Error::Revision);
        }
        if s.viewer_devices.iter().any(|id| !identifier(id)) {
            return Err(Error::Membership);
        }
        Ok(Self {
            identity: Identity {
                session: s.session_id,
                source_device: s.source_device_id,
                source_hook: s.source_hook_id,
                epoch: wire.epoch,
            },
            revision: s.revision,
            viewers: s.viewer_devices,
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// attachment 返回前后来源和 epoch 必须固定；续期不可偷偷变成另一个来源的加入。
    pub fn matches(&self, expected: &Identity) -> Result<(), Error> {
        if &self.identity == expected {
            Ok(())
        } else {
            Err(Error::Identity)
        }
    }
    /// 已移除成员不得借自动续期重新加入；首次显式加入不调用此检查。
    pub fn renewal(&self, expected: &Identity, device: &str) -> Result<(), Error> {
        self.matches(expected)?;
        if !identifier(device) || !self.viewers.iter().any(|v| v == device) {
            return Err(Error::Membership);
        }
        Ok(())
    }
}
#[path = "live_relay_viewer_bootstrap.rs"]
mod bootstrap;
pub use bootstrap::join_cursor;
#[path = "live_relay_control_events.rs"]
mod events;
pub use events::{ControlBatch, ControlCursor, ControlEvent};
#[path = "live_relay_viewer_terminal.rs"]
mod terminal;
pub use terminal::terminal_snapshot;
#[cfg(test)]
#[path = "live_relay_viewer_identity_tests.rs"]
mod tests;
