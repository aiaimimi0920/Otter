//! 传统签名邀请的有界形状与来源校验；签名真实性仍由获认证的 Loom 验证，不在本地臆造授权。
use super::Error;
use serde::{Deserialize, Serialize};
#[path = "projection_sender_signing.rs"]
mod signing;

pub(super) const MAX_REVISION: u64 = 9_007_199_254_740_991;
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Envelope {
    protocol: String,
    projection_id: String,
    server_origin: String,
    source: Source,
    content: Content,
    expires_at_ms: u64,
    nonce: String,
    signature: Signature,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    device_id: String,
    session_id: String,
    unit_id: String,
    revision: u64,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Content {
    kind: String,
    digest: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Signature {
    algorithm: String,
    key_id: String,
    value: String,
}
impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 邀请可能作为后续接收凭据，Debug 不输出签名、nonce 或完整 wire。
        f.debug_struct("ProjectionEnvelope")
            .field("projection_id", &self.projection_id)
            .finish_non_exhaustive()
    }
}
impl Envelope {
    pub fn projection_id(&self) -> &str {
        &self.projection_id
    }
    pub fn origin(&self) -> &str {
        &self.server_origin
    }
    pub fn source_device(&self) -> &str {
        &self.source.device_id
    }
    pub fn source_revision(&self) -> u64 {
        self.source.revision
    }
    pub(crate) fn source_session(&self) -> &str {
        &self.source.session_id
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
    pub(super) fn validate(&self, origin: &str) -> Result<(), Error> {
        if self.protocol != "neuro.qr-projection.v1"
            || !self
                .projection_id
                .strip_prefix("projection:")
                .is_some_and(|id| id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit()))
            || canonical_origin(&self.server_origin).as_deref() != Ok(self.server_origin.as_str())
            || !identifier(&self.source.device_id)
            || !identifier(&self.source.session_id)
            || !identifier(&self.source.unit_id)
            || !(1..=MAX_REVISION).contains(&self.source.revision)
            || !matches!(self.content.kind.as_str(), "sticker" | "art")
            || !digest(&self.content.digest)
            || self.nonce.len() != 32
            || !self.nonce.bytes().all(|c| c.is_ascii_hexdigit())
            || !(1..=MAX_REVISION).contains(&self.expires_at_ms)
            || self.signature.algorithm != "ed25519"
            || self.signature.key_id != self.source.device_id
            || self.signature.value.len() != 86
            || !self
                .signature
                .value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        {
            return Err(Error::Envelope);
        }
        if self.server_origin != origin {
            return Err(Error::Origin);
        }
        Ok(())
    }
}
pub(super) fn canonical_origin(value: &str) -> Result<String, Error> {
    if value.len() > 256 || !value.is_ascii() || value.contains('\\') {
        return Err(Error::Origin);
    }
    crate::device_session_attempt::canonical_origin(value).map_err(|_| Error::Origin)
}
pub(super) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:/-".contains(&c))
}
pub(super) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
