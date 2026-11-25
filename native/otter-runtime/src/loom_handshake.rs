//! Native-capability handshake data boundary; retain only a bounded catalog, without automatic capability execution or remote-data logging.
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
const PROTOCOL: &str = "loom.hook.v1";
pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    TooLarge,
    Encoding,
    Remote,
    Protocol,
    Identity,
    Transport,
    Capabilities,
}
#[derive(Clone, Copy, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Websocket,
    SharedMemory,
}
/// Validated immutable catalog; recovery threads can reuse the same snapshot without copying capability definitions.
#[derive(Clone)]
pub struct Handshake(Arc<Parsed>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Parsed {
    protocol_version: String,
    server_name: String,
    server_version: String,
    session_id: String,
    transport: Transport,
    capabilities: Capabilities,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Capabilities {
    art_definitions: Vec<Value>,
    operations: Vec<String>,
    surface: Value,
}
impl Handshake {
    pub fn arts(&self) -> &[Value] {
        &self.0.capabilities.art_definitions
    }
    pub fn capability(&self, index: usize) -> Option<crate::loom_capability::Capability<'_>> {
        crate::loom_capability::Capability::new(self.arts().get(index)?)
    }
    pub fn session_id(&self) -> &str {
        &self.0.session_id
    }
    pub fn transport(&self) -> Transport {
        self.0.transport
    }
    pub fn server_name(&self) -> &str {
        &self.0.server_name
    }
    pub fn server_version(&self) -> &str {
        &self.0.server_version
    }
    pub fn operations(&self) -> &[String] {
        &self.0.capabilities.operations
    }
    pub fn surface(&self) -> &Value {
        &self.0.capabilities.surface
    }
}
fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
pub fn request(client_id: &str, version: &str) -> Result<String, Error> {
    if !text(client_id) || !text(version) {
        return Err(Error::Identity);
    }
    Ok(serde_json::json!({"method":"loom.hook.handshake","params":{
        "protocolVersion":PROTOCOL,
        "clientId":client_id,"clientVersion":version,"platform":"windows","deviceId":"device:local",
        "transports":["websocket","shared_memory"]}})
    .to_string())
}
/// method events are not handshake responses; reject other invalid formats/errors to avoid replacing the displayed catalog with an invalid response.
pub fn response(bytes: &[u8]) -> Result<Option<Handshake>, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let envelope: Value = serde_json::from_slice(bytes).map_err(|_| Error::Encoding)?;
    if envelope.get("method").is_some() {
        return Ok(None);
    }
    if envelope.get("error").is_some_and(|error| !error.is_null()) {
        return Err(Error::Remote);
    }
    // Decode again from the original text to preserve serde's rejection of duplicate known fields, rather than decoding after Value overwrites them.
    let result: Parsed = serde_json::from_slice(bytes).map_err(|_| Error::Encoding)?;
    if result.protocol_version != PROTOCOL {
        return Err(Error::Protocol);
    }
    if !text(&result.server_name) || !text(&result.server_version) || !text(&result.session_id) {
        return Err(Error::Identity);
    }
    let caps = &result.capabilities;
    if caps.art_definitions.len() > 512
        || caps.operations.len() > 512
        || caps.operations.iter().any(|value| !text(value))
    {
        return Err(Error::Capabilities);
    }
    for name in ["runtimes", "nodes", "transports", "capabilities"] {
        let entries = caps.surface[name].as_array().ok_or(Error::Capabilities)?;
        if entries.len() > 512
            || entries
                .iter()
                .any(|entry| !entry.as_str().is_some_and(text))
        {
            return Err(Error::Capabilities);
        }
    }
    if !caps.surface["apiVersion"].as_str().is_some_and(text)
        || ["pointer", "hover", "touch", "keyboard"]
            .iter()
            .any(|name| !caps.surface["input"][name].is_boolean())
    {
        return Err(Error::Capabilities);
    }
    for (index, art) in caps.art_definitions.iter().enumerate() {
        let id = art["id"]
            .as_str()
            .filter(|id| text(id))
            .ok_or(Error::Capabilities)?;
        if caps.art_definitions[..index].iter().any(|other| {
            other["id"]
                .as_str()
                .is_some_and(|other| other.trim() == id.trim())
        }) {
            return Err(Error::Capabilities);
        }
        // As in the baseline, missing/empty labels fall back to id; valid labels still undergo text-budget and control-character checks.
        if art["label"]
            .as_str()
            .is_some_and(|label| !label.trim().is_empty() && !text(label))
        {
            return Err(Error::Capabilities);
        }
        for name in ["parameters", "inputs", "outputs"] {
            let entries = art[name].as_array().ok_or(Error::Capabilities)?;
            if entries.len() > 128 || entries.iter().any(|entry| !entry.is_object()) {
                return Err(Error::Capabilities);
            }
        }
    }
    Ok(Some(Handshake(Arc::new(result))))
}
#[cfg(any(test, feature = "test-support"))]
#[path = "loom_handshake_tests.rs"]
pub mod tests;
