//! 本地恢复提示而非授权/服务端回执；禁止从磁盘记录直接构造可操作的 Binding。
use crate::{
    surface_attach_runtime::Request,
    surface_live_binding::{Binding, Lifecycle},
};
use serde::{Deserialize, Serialize};
use std::io;

pub const MAX_BYTES: usize = 64 * 1024;
pub const MAX_RECORDS: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    origin: String,
    device: String,
    node: String,
    art: Option<String>,
    binding: Option<Hint>,
    uncertain: bool,
    in_flight: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Hint {
    instance: String,
    attachment: String,
    generation: u64,
    lifecycle: Option<Lifecycle>,
    revision: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    records: Vec<Record>,
}
impl Record {
    pub fn from_binding(
        binding: &Binding,
        art: Option<&str>,
        uncertain: bool,
        in_flight: bool,
    ) -> Self {
        Self {
            origin: binding.origin().into(),
            device: binding.device().into(),
            node: binding.node().into(),
            art: art.map(Into::into),
            uncertain,
            in_flight,
            binding: Some(Hint {
                instance: binding.instance().into(),
                attachment: binding.attachment().into(),
                generation: binding.generation(),
                lifecycle: Some(binding.lifecycle()),
                revision: Some(binding.lifecycle_revision()),
            }),
        }
    }
    pub fn from_request(origin: &str, device: &str, request: &Request, uncertain: bool) -> Self {
        Self {
            origin: origin.into(),
            device: device.into(),
            node: request.node().into(),
            art: Some(request.art().into()),
            uncertain,
            in_flight: false,
            binding: request.recovery_candidate().map(|candidate| Hint {
                instance: candidate.instance().into(),
                attachment: candidate.attachment().into(),
                generation: candidate.generation(),
                lifecycle: None,
                revision: None,
            }),
        }
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn device(&self) -> &str {
        &self.device
    }
    pub fn node(&self) -> &str {
        &self.node
    }
    pub fn art(&self) -> Option<&str> {
        self.art.as_deref()
    }
    pub fn uncertain(&self) -> bool {
        self.uncertain
    }
    pub fn in_flight(&self) -> bool {
        self.in_flight
    }
    pub(crate) fn has_binding(&self) -> bool {
        self.binding.is_some()
    }
    pub(crate) fn validate(&self) -> io::Result<()> {
        use crate::live_relay_viewer_identity::identifier;
        if self.origin.len() > 2048
            || crate::device_session_attempt::canonical_origin(&self.origin)
                .ok()
                .as_deref()
                != Some(&self.origin)
            || !identifier(&self.device)
            || !identifier(&self.node)
            || self.art.as_deref().is_some_and(|art| !identifier(art))
            || self.binding.as_ref().is_some_and(|binding| {
                !identifier(&binding.instance)
                    || !identifier(&binding.attachment)
                    || binding.lifecycle.is_some() != binding.revision.is_some()
            })
        {
            return Err(invalid());
        }
        Ok(())
    }
}
pub fn encode(records: &[Record]) -> io::Result<Vec<u8>> {
    if records.len() > MAX_RECORDS {
        return Err(invalid());
    }
    for record in records {
        record.validate()?;
    }
    let bytes = serde_json::to_vec(&Envelope {
        version: 1,
        records: records.to_vec(),
    })
    .map_err(io::Error::other)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> io::Result<Vec<Record>> {
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    if envelope.version != 1 || envelope.records.len() > MAX_RECORDS {
        return Err(invalid());
    }
    for record in &envelope.records {
        record.validate()?;
    }
    Ok(envelope.records)
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid Surface recovery metadata",
    )
}

#[cfg(test)]
#[path = "surface_recovery_record_tests.rs"]
mod tests;
