//! Identity/renewal prechecks for authenticated HTTP responses; successful parsing is not authentication, and media-side snapshots are unacceptable.
use serde::Deserialize;
pub const PROTOCOL: &str = "loom.live.v1";
pub const MAX_JSON: usize = 1024 * 1024;
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
pub fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    if bytes.is_empty() || bytes.len() > MAX_JSON {
        return Err(Error::Limit);
    }
    serde_json::from_slice(bytes).map_err(|_| Error::Json)
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
impl Snapshot {
    /// HTTP transport must first verify the origin and Device credentials and bound reads; no keys are held here.
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
    /// The source and epoch must remain fixed across the attachment response; renewal must not silently become a join to another source.
    pub fn matches(&self, expected: &Identity) -> Result<(), Error> {
        if &self.identity == expected {
            Ok(())
        } else {
            Err(Error::Identity)
        }
    }
    /// Removed members must not rejoin through automatic renewal; the first explicit join does not call this check.
    pub fn renewal(&self, expected: &Identity, device: &str) -> Result<(), Error> {
        self.matches(expected)?;
        if !identifier(device) || !self.viewers.iter().any(|v| v == device) {
            return Err(Error::Membership);
        }
        Ok(())
    }
}
#[path = "live_relay_viewer_bootstrap.rs"]
pub mod bootstrap;
pub use bootstrap::join_cursor;
#[path = "live_relay_control_events.rs"]
pub mod events;
pub use events::{ControlBatch, ControlCursor, ControlEvent};
#[path = "live_relay_viewer_terminal.rs"]
pub mod terminal;
pub use terminal::terminal_snapshot;
#[cfg(test)]
#[path = "live_relay_viewer_identity_tests.rs"]
mod tests;
