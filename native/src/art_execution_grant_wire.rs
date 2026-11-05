//! loom.art.local.grant.v1 的客户端 wire DTO；协议归 Loom，不导入其私有实现。
use serde::{Deserialize, Serialize};
pub const PROTOCOL: &str = "loom.art.local.grant.v1";
pub const SDK: &str = "otter.art.v1";
pub const DOMAIN: &[u8] = b"loom.art.local.grant.v1\0";
pub const MAX_ENVELOPE: usize = 24 * 1024;
pub const MAX_PAYLOAD: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub origin: String,
    pub request_id: String,
    pub device_id: String,
    pub workspace_id: String,
    pub instance_id: String,
    pub generation: u64,
    pub art_id: String,
    pub art_version: String,
    pub package_digest: String,
    pub sdk_version: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    LocalState,
    Preview,
    LoomService,
    WorkspaceResult,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub resource_id: String,
    pub digest: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Grant {
    pub protocol_version: String,
    pub grant_id: String,
    pub binding: Binding,
    pub execution_owner: String,
    pub result_owner: String,
    pub permissions: Vec<Permission>,
    pub services: Vec<String>,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub offline_until_ms: u64,
    pub source: Source,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Envelope {
    pub algorithm: String,
    pub key_id: String,
    pub payload: String,
    pub signature: String,
}
