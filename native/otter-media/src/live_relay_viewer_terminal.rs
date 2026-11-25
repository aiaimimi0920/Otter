//! Closure facts from authoritative member snapshots; HTTP 404/network errors alone do not mean the session is closed.
use super::{identifier, parse, Error, Identity, PROTOCOL};
use serde::Deserialize;
#[derive(Deserialize)]
struct Terminal {
    session: Session,
    epoch: u64,
    closed: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    protocol_version: String,
    session_id: String,
    viewer_devices: Vec<String>,
}
/// Call only for successful responses from an authenticated member endpoint; do not accept media messages or identity-free HTTP error bodies.
pub fn terminal_snapshot(bytes: &[u8], expected: &Identity, device: &str) -> Result<bool, Error> {
    let response: Terminal = parse(bytes)?;
    if !identifier(device)
        || response.session.protocol_version != PROTOCOL
        || response.session.session_id != expected.session()
        || response.epoch < expected.epoch()
        || !response.session.viewer_devices.iter().any(|v| v == device)
    {
        return Err(Error::Identity);
    }
    Ok(response.closed)
}
