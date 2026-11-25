//! Replay from the position immediately before this join event; do not blindly trust next or skip revocation events after joining.
use super::events::Response;
use super::{identifier, parse, Error, Snapshot, PROTOCOL};
use serde::Deserialize;
#[derive(Deserialize)]
struct Joined {
    revision: u64,
    reason: Option<String>,
    viewers: Vec<String>,
}
pub fn join_cursor(attached: &Snapshot, device: &str, bytes: &[u8]) -> Result<u64, Error> {
    if !identifier(device) {
        return Err(Error::Identity);
    }
    let response: Response = parse(bytes)?;
    if response.protocol_version != PROTOCOL {
        return Err(Error::Identity);
    }
    let identity = attached.identity();
    let mut start = None;
    for (index, event) in response.events.iter().enumerate() {
        if event.protocol_version != PROTOCOL
            || event.session_id != identity.session()
            || event.epoch != identity.epoch()
            || event.message_type != "session_state"
        {
            continue;
        }
        let Ok(joined) = serde_json::from_str::<Joined>(event.payload.get()) else {
            continue;
        };
        if joined.revision == attached.revision()
            && joined.reason.as_deref() == Some("viewer_joined")
            && joined.viewers.iter().any(|id| id == device)
        {
            start = Some(index);
            break;
        }
    }
    let start = start.ok_or(Error::History)?;
    let cursor = response.events[start]
        .sequence
        .checked_sub(1)
        .ok_or(Error::History)?;
    let mut next = cursor;
    // reset may indicate eviction of pre-join history, but the complete contiguous suffix starting at this join must be present.
    for event in &response.events[start..] {
        if event.protocol_version != PROTOCOL
            || event.session_id != identity.session()
            || event.epoch != identity.epoch()
            || next.checked_add(1) != Some(event.sequence)
        {
            return Err(Error::History);
        }
        next = event.sequence;
    }
    if next != response.next {
        return Err(Error::History);
    }
    Ok(cursor)
}
