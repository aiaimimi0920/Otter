//! 经验证的单附件值；宿主显隐和摘要不属于 invocation，不能通过 Deserialize 绕过边界。
use crate::loom_extension_snapshot::{text, MAX_INTEGER};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::sync::Arc;
pub(crate) const PAYLOAD_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Wire,
    Limit,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResourceKind {
    File,
    SharedImage,
    SharedMemory,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Resource {
    resource_id: String,
    kind: ResourceKind,
    digest: String,
    byte_length: u64,
    lease_id: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    attachment_id: String,
    type_id: String,
    schema_version: String,
    revision: u64,
    plugin_id: String,
    plugin_version: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_string"
    )]
    renderer_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_payload"
    )]
    payload: Option<Value>,
    #[serde(default)]
    resource_refs: Vec<Resource>,
}
fn present_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn present_payload<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(d).map(Some)
}
pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Clone)]
pub struct Attachment(Arc<Data>);
impl Serialize for Attachment {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl Attachment {
    pub fn parse(value: Value) -> Result<Self, Error> {
        let result: Data = serde_json::from_value(value).map_err(|_| Error::Wire)?;
        if !text(&result.attachment_id, 384)
            || !text(&result.type_id, 384)
            || !text(&result.schema_version, 64)
            || !text(&result.plugin_id, 384)
            || !text(&result.plugin_version, 128)
            || result.revision > MAX_INTEGER
            || !result
                .attachment_id
                .starts_with(&format!("{}.", result.plugin_id))
            || result.renderer_id.as_ref().is_some_and(|id| !text(id, 384))
            || result.resource_refs.len() > 16
        {
            return Err(Error::Wire);
        }
        if result.payload.is_none() && result.resource_refs.is_empty() {
            return Err(Error::Wire);
        }
        if let Some(payload) = &result.payload {
            crate::unit_attachment_json::bounded(payload, PAYLOAD_BYTES)
                .map_err(|_| Error::Limit)?;
        }
        for resource in &result.resource_refs {
            if !valid_digest(&resource.digest)
                || resource.resource_id != format!("sha256:{}", resource.digest)
                || !text(&resource.lease_id, 384)
                || resource.byte_length > 64 * 1024 * 1024
            {
                return Err(Error::Wire);
            }
        }
        Ok(Self(Arc::new(result)))
    }
    pub fn id(&self) -> &str {
        &self.0.attachment_id
    }
    pub fn type_id(&self) -> &str {
        &self.0.type_id
    }
    pub fn schema(&self) -> &str {
        &self.0.schema_version
    }
    pub fn revision(&self) -> u64 {
        self.0.revision
    }
    pub fn plugin(&self) -> &str {
        &self.0.plugin_id
    }
    pub fn plugin_version(&self) -> &str {
        &self.0.plugin_version
    }
    pub fn renderer(&self) -> Option<&str> {
        self.0.renderer_id.as_deref()
    }
    pub fn payload(&self) -> Option<&Value> {
        self.0.payload.as_ref()
    }
}
