//! Declarative input to bounded wire events; encoding is not authorization, and the sending owner must recheck leases and current revision.
use crate::{surface_click_route::ClickAction, surface_live_binding::Binding};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{ser::SerializeMap, Serialize, Serializer};
use std::io::{self, Write};

pub const MAX_EVENT_BYTES: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Modifiers {
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Revision,
    Random,
    Encoding,
}
/// No deserialization or mutable body; a single event reuses its original bytes/ID instead of generating a new action identity on retry.
pub struct ClickEvent {
    id: String,
    bytes: Vec<u8>,
    binding: Binding,
    revision: u64,
    attempted: bool,
    // Set only after parent-side JS authority validation; not part of the wire, and remote responses cannot upgrade the default runtime.
    javascript: bool,
}
impl ClickEvent {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn attempted(&self) -> bool {
        self.attempted
    }
    pub(crate) fn is_javascript(&self) -> bool {
        self.javascript
    }
    pub(crate) fn javascript(mut self) -> Self {
        self.javascript = true;
        self
    }
    pub(crate) fn begin_submission(&mut self) {
        self.attempted = true;
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Wire<'a, P> {
    protocol_version: &'static str,
    instance_id: &'a str,
    attachment_id: &'a str,
    event_id: &'a str,
    node_id: &'a str,
    event: &'a str,
    action: &'a str,
    class: &'a str,
    generation: u64,
    base_revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    modifiers: Option<Modifiers>,
    payload: P,
}
/// For pure wire encoding only; does not authorize ordinary Surface mounts or Wall input.
#[derive(Clone, Copy)]
pub struct Identity<'a> {
    pub instance: &'a str,
    pub attachment: &'a str,
    pub generation: u64,
}
impl<'a> From<&'a Binding> for Identity<'a> {
    fn from(binding: &'a Binding) -> Self {
        Self {
            instance: binding.instance(),
            attachment: binding.attachment(),
            generation: binding.generation(),
        }
    }
}
impl ClickAction<'_> {
    pub(crate) fn includes_surface_values(&self) -> bool {
        self.node.props().get("includeSurfaceValues") == Some(&serde_json::Value::Bool(true))
    }

    /// Callers without forms retain the original payload; production hosts use encode_with_values.
    pub fn encode(
        &self,
        binding: &Binding,
        revision: u64,
        modifiers: Modifiers,
    ) -> Result<ClickEvent, Error> {
        self.encode_with_values(binding, revision, modifiers, &Default::default())
    }
    pub fn encode_with_values(
        &self,
        binding: &Binding,
        revision: u64,
        modifiers: Modifiers,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<ClickEvent, Error> {
        let (id, bytes) = self.encode_identity(binding.into(), revision, modifiers, values)?;
        Ok(ClickEvent {
            id,
            bytes,
            binding: binding.clone(),
            revision,
            attempted: false,
            javascript: false,
        })
    }
    pub(crate) fn encode_identity(
        &self,
        identity: Identity<'_>,
        revision: u64,
        modifiers: Modifiers,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(String, Vec<u8>), Error> {
        let empty = serde_json::Value::Object(Default::default());
        let payload = self
            .node
            .props()
            .get("eventPayload")
            .filter(|value| !value.is_null())
            .unwrap_or(&empty);
        let values = (self.includes_surface_values() && !values.is_empty()).then_some(values);
        encode_identity(
            identity,
            revision,
            Declaration {
                node_id: self.node.id(),
                event: "click",
                action: self.action,
                class: "discrete",
                modifiers: (self.node.kind() != crate::surface_scene::Kind::Button)
                    .then_some(modifiers),
                payload: ClickPayload { payload, values },
            },
        )
    }
}
/// Borrow configuration and form values to write the final bounded output directly, without first materializing a second JSON tree.
struct ClickPayload<'a> {
    payload: &'a serde_json::Value,
    values: Option<&'a serde_json::Map<String, serde_json::Value>>,
}
impl Serialize for ClickPayload<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some(values) = self.values else {
            return self.payload.serialize(serializer);
        };
        let mut map = serializer.serialize_map(None)?;
        let mut inserted = false;
        if let Some(object) = self.payload.as_object() {
            // The current serde_json Map sorts by key; replace reserved keys in place and prohibit duplicate output keys.
            for (key, value) in object {
                if !inserted && key.as_str() >= "surfaceValues" {
                    map.serialize_entry("surfaceValues", values)?;
                    inserted = true;
                }
                if key != "surfaceValues" {
                    map.serialize_entry(key, value)?;
                }
            }
        } else {
            map.serialize_entry("configured", self.payload)?;
        }
        if !inserted {
            map.serialize_entry("surfaceValues", values)?;
        }
        map.end()
    }
}
/// Each input type supplies declarations after owner validation, sharing immutable IDs/bytes and one serialization budget.
pub(crate) struct Declaration<'a, P = &'a serde_json::Value> {
    pub node_id: &'a str,
    pub event: &'a str,
    pub action: &'a str,
    pub class: &'static str,
    pub modifiers: Option<Modifiers>,
    pub payload: P,
}
pub(crate) fn encode(
    binding: &Binding,
    revision: u64,
    declaration: Declaration<'_>,
) -> Result<ClickEvent, Error> {
    let (id, bytes) = encode_identity(binding.into(), revision, declaration)?;
    Ok(ClickEvent {
        id,
        bytes,
        binding: binding.clone(),
        revision,
        attempted: false,
        javascript: false,
    })
}
pub(crate) fn encode_identity<P: Serialize>(
    identity: Identity<'_>,
    revision: u64,
    declaration: Declaration<'_, P>,
) -> Result<(String, Vec<u8>), Error> {
    if revision == 0 {
        return Err(Error::Revision);
    }
    if identity.generation > 9_007_199_254_740_991
        || !crate::live_relay_viewer_identity::identifier(identity.instance)
        || !crate::live_relay_viewer_identity::identifier(identity.attachment)
    {
        return Err(Error::Encoding);
    }
    let mut random = [0_u8; 16];
    SystemRandom::new()
        .fill(&mut random)
        .map_err(|_| Error::Random)?;
    let id = format!("event:{}", URL_SAFE_NO_PAD.encode(random));
    let wire = Wire {
        protocol_version: "loom.surface.v1",
        instance_id: identity.instance,
        attachment_id: identity.attachment,
        event_id: &id,
        node_id: declaration.node_id,
        event: declaration.event,
        action: declaration.action,
        class: declaration.class,
        generation: identity.generation,
        base_revision: revision,
        modifiers: declaration.modifiers,
        payload: declaration.payload,
    };
    let mut output = Bounded(Vec::new());
    serde_json::to_writer(&mut output, &wire).map_err(|_| Error::Encoding)?;
    Ok((id, output.0))
}
struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_EVENT_BYTES.saturating_sub(self.0.len()) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let needed = self.0.len() + bytes.len();
        if needed > self.0.capacity() {
            // Bounded geometric growth prevents small serializer writes from triggering byte-by-byte reallocations.
            let capacity = needed
                .max(self.0.capacity().saturating_mul(2))
                .clamp(512, MAX_EVENT_BYTES);
            self.0
                .try_reserve_exact(capacity - self.0.len())
                .map_err(|_| io::ErrorKind::OutOfMemory)?;
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_click_event_tests.rs"]
mod tests;
