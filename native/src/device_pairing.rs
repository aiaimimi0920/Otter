//! Explicit pairing registration sends only the public key and saves association; registration responses are never approval/session credentials.
use crate::{
    device_identity::{valid_field, Identity},
    device_identity_store::{Loaded, Store},
    device_session_client::Client,
    loom_bridge_stream::Interrupt,
};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};

const PATH: &str = "/v1/devices/requests";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Name,
    Key,
    Limit,
    Http(u16),
    Json,
    Missing,
    Ambiguous,
    Device,
    Stopped,
    Network(crate::device_session_client::Error),
    Store(crate::device_identity_store::Error),
}
struct Request {
    public: String,
    body: Vec<u8>,
}
impl Request {
    fn new(identity: &Identity, name: &str) -> Result<Self, Error> {
        let name = name.trim();
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
            return Err(Error::Name);
        }
        let public = identity.public_key().map_err(|_| Error::Key)?;
        let body = serde_json::to_vec(&serde_json::json!({
            "name": name, "kind": "computer", "address": format!("hook://{name}"),
            "publicKey": public,
        }))
        .map_err(|_| Error::Json)?;
        Ok(Self { public, body })
    }
    fn finish(self, status: u16, body: &[u8]) -> Result<String, Error> {
        if body.len() > crate::device_session_attempt::MAX_RESPONSE {
            return Err(Error::Limit);
        }
        if !(200..300).contains(&status) {
            return Err(Error::Http(status));
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Device {
            id: String,
            #[serde(default)]
            public_key: Option<String>,
        }
        #[derive(Deserialize)]
        struct Registry {
            #[serde(default)]
            devices: Vec<Device>,
            #[serde(default)]
            pending: Vec<Device>,
        }
        let registry: Registry = serde_json::from_slice(body).map_err(|_| Error::Json)?;
        let mut selected = None;
        for device in registry.pending.into_iter().chain(registry.devices) {
            if device.public_key.as_deref() != Some(self.public.as_str()) {
                continue;
            }
            if !valid_field(&device.id, 160) {
                return Err(Error::Device);
            }
            // Do not guess the association target when the same public key has different IDs; duplicate records with the same ID are acceptable.
            if selected.as_ref().is_some_and(|id| id != &device.id) {
                return Err(Error::Ambiguous);
            }
            selected = Some(device.id);
        }
        selected.ok_or(Error::Missing)
    }
}

/// Obtain only the public-key association returned by this endpoint, without changing default identity; for connection-specific associations.
pub fn register(
    client: &Client,
    identity: &Identity,
    name: &str,
    stop: &AtomicBool,
    mut connected: impl FnMut(Interrupt),
) -> Result<String, Error> {
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    let request = Request::new(identity, name)?;
    let (status, body) = client
        .post(PATH, &request.body, stop, &mut connected)
        .map_err(Error::Network)?;
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    request.finish(status, &body)
}

/// Explicit default-association pairing. Callers first obtain identity via Store::load_or_create; no key generation here.
/// After success, upper layers must revoke this origin's old authorization; the return value indicates only device-ID change, not approval.
pub fn register_and_save(
    client: &Client,
    store: &Store,
    loaded: &mut Loaded,
    name: &str,
    stop: &AtomicBool,
    connected: impl FnMut(Interrupt),
) -> Result<bool, Error> {
    let device = register(client, loaded.identity(), name, stop, connected)?;
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    let changed = loaded.identity().device_id() != Some(device.as_str());
    // After the commit checkpoint, finish the file transaction without mid-write cancellation; failure does not mutate the caller's Loaded.
    // Network registration may succeed while local saving fails; report Store error rather than pairing completion.
    store.bind_and_save(loaded, &device).map_err(Error::Store)?;
    Ok(changed)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "device_pairing_tests.rs"]
mod tests;
