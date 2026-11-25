//! LAN sharing carries stable source Loom IDs; receive requests always target the currently authenticated Loom.
use super::{envelope, Envelope, Error, Invitation};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
const LINK: &str = "otter://projection/lan#";
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Link {
    pub source_loom_id: String,
    pub envelope: Envelope,
}
pub(crate) fn loom_id(value: &str) -> bool {
    value.strip_prefix("loom-").is_some_and(envelope::digest)
}
pub fn encode(envelope: &Envelope, source_loom_id: &str) -> Result<String, Error> {
    envelope.validate(envelope.origin())?;
    if !loom_id(source_loom_id) {
        return Err(Error::Entry);
    }
    let bytes = serde_json::to_vec(&Link {
        source_loom_id: source_loom_id.into(),
        envelope: envelope.clone(),
    })
    .map_err(|_| Error::Json)?;
    Ok(format!("{LINK}{}", URL_SAFE_NO_PAD.encode(bytes)))
}
pub fn parse(text: &str) -> Result<Link, Error> {
    if text.len() > 8192 {
        return Err(Error::Limit);
    }
    let payload = text.trim().strip_prefix(LINK).ok_or(Error::Envelope)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| Error::Envelope)?;
    let link: Link = serde_json::from_slice(&bytes).map_err(|_| Error::Json)?;
    link.envelope.validate(link.envelope.origin())?;
    if !loom_id(&link.source_loom_id) {
        return Err(Error::Entry);
    }
    Ok(link)
}
#[cfg(test)]
#[path = "projection_share_tests.rs"]
mod tests;

pub struct Import {
    pub envelope: Envelope,
    pub source_loom_id: String,
    pub cancel: std::sync::Arc<AtomicBool>,
}
impl Import {
    pub(crate) fn current(&self) -> bool {
        !self.cancel.load(Ordering::Acquire)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Preview<'a> {
    envelope: Envelope,
    revision: u64,
    digest: String,
    width: u32,
    height: u32,
    linked: bool,
    receiver_device_id: Option<String>,
    receiver_unit_id: Option<String>,
    source_name: Option<String>,
    #[serde(borrow)]
    snapshot: super::sender::snapshot::Wire<'a>,
    delivery: Option<Delivery>,
    route: Option<String>,
    peer_id: Option<String>,
    offline_transport: Option<serde_json::Value>,
    source_loom_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Delivery {
    target_device_id: String,
    status: String,
}
#[cfg(test)]
pub(crate) fn inspected(
    bytes: &[u8],
    expected: &Envelope,
    origin: &str,
    device: &str,
    cancel: &AtomicBool,
) -> Result<Invitation, Error> {
    inspected_from(bytes, expected, origin, device, None, cancel)
}
pub(crate) fn inspected_from(
    bytes: &[u8],
    expected: &Envelope,
    origin: &str,
    device: &str,
    source_loom_id: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Invitation, Error> {
    expected.validate(expected.origin())?;
    if bytes.len() > super::accept::MAX_RESPONSE {
        return Err(Error::Limit);
    }
    let wire: Preview<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Response)?;
    let offline = if wire.route.as_deref() == Some("offline_peer") {
        let source = source_loom_id.ok_or(Error::Response)?;
        if wire.peer_id.as_deref() != Some(source) || wire.delivery.is_none() {
            return Err(Error::Response);
        }
        Some(super::receive_route::Offline::new(origin, source)?)
    } else {
        // Only the authenticated import response may bind a shared Loom alias to this transport.
        if source_loom_id.is_none() {
            expected.validate(origin)?;
        }
        if wire.peer_id.is_some() || wire.route.as_deref().is_some_and(|r| r != "shared_loom") {
            return Err(Error::Response);
        }
        None
    };
    let receiver_unit = format!("delivery-{}", &expected.projection_id()[11..]);
    if (offline.is_none() && expected.source_device() == device)
        || wire.envelope != *expected
        || !wire.linked
        || source_loom_id.is_some_and(|id| wire.source_loom_id.as_deref() != Some(id))
        || wire.offline_transport.is_some()
        || !(expected.source_revision()..=envelope::MAX_REVISION).contains(&wire.revision)
        || !envelope::digest(&wire.digest)
        || (wire.revision == expected.source_revision() && wire.digest != expected.content_digest())
        || wire.receiver_device_id.is_some() != wire.receiver_unit_id.is_some()
        || wire
            .receiver_device_id
            .as_deref()
            .is_some_and(|id| id != device)
        || wire
            .receiver_unit_id
            .as_deref()
            .is_some_and(|id| id != receiver_unit)
        || wire.delivery.as_ref().is_some_and(|d| {
            d.target_device_id != device
                || !matches!(
                    d.status.as_str(),
                    "awaiting_confirmation" | "accepted" | "displayed"
                )
        })
    {
        return Err(Error::Response);
    }
    let digest = super::sender::snapshot::Snapshot::inspect(&wire.snapshot, cancel)
        .map_err(|_| Error::Snapshot)?;
    if digest != wire.digest
        || (wire.snapshot.width, wire.snapshot.height) != (wire.width, wire.height)
    {
        return Err(Error::Snapshot);
    }
    let name = wire
        .source_name
        .unwrap_or_else(|| expected.source_device().into());
    if name.len() > 1024 || name.chars().any(char::is_control) {
        return Err(Error::Response);
    }
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    Ok(Invitation {
        envelope: expected.clone(),
        revision: wire.revision,
        digest: wire.digest,
        source_name: name,
        accepted: wire.receiver_device_id.is_some(),
        offline,
        shared_origin: (wire.delivery.is_none() && source_loom_id.is_some()).then(|| origin.into()),
        public_invitation: wire.delivery.is_none(),
    })
}
