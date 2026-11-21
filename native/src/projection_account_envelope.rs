//! v2 邀请的本地有界验签；签名正确不等于账号授权，后者只由本机 Loom 执行。
use super::*;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
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
    account_id: String,
    public_key: String,
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
        f.debug_struct("AccountProjectionEnvelope")
            .field("projection_id", &self.projection_id)
            .finish_non_exhaustive()
    }
}
impl Envelope {
    /// JSON 或 v2 链接仅本地解析；不请求 QR 地址，不自动登录或接受邀请。
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.len() > 8192 {
            return Err(Error::Envelope);
        }
        let text = text.trim();
        let decoded;
        let json = if let Some(encoded) = text.strip_prefix("hook://projection/v2#") {
            decoded = URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| Error::Envelope)?;
            if URL_SAFE_NO_PAD.encode(&decoded) != encoded {
                return Err(Error::Envelope);
            }
            std::str::from_utf8(&decoded).map_err(|_| Error::Envelope)?
        } else {
            text
        };
        if json.len() > 4096 {
            return Err(Error::Envelope);
        }
        let value: Self = serde_json::from_str(json).map_err(|_| Error::Envelope)?;
        value.validate()?;
        Ok(value)
    }
    pub fn link(&self) -> Result<String, Error> {
        self.validate()?;
        let json = serde_json::to_vec(self).map_err(|_| Error::Envelope)?;
        if json.len() > 4096 {
            return Err(Error::Envelope);
        }
        Ok(format!(
            "hook://projection/v2#{}",
            URL_SAFE_NO_PAD.encode(json)
        ))
    }
    pub fn projection_id(&self) -> &str {
        &self.projection_id
    }
    pub fn origin(&self) -> &str {
        &self.server_origin
    }
    pub fn source_device(&self) -> &str {
        &self.source.device_id
    }
    pub fn source_session(&self) -> &str {
        &self.source.session_id
    }
    pub fn source_unit(&self) -> &str {
        &self.source.unit_id
    }
    pub fn digest(&self) -> &str {
        &self.content.digest
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
    pub(super) fn kind(&self) -> &str {
        &self.content.kind
    }
    pub(super) fn matches_origin(&self, context: &Context) -> bool {
        self.server_origin == context.origin
    }
    pub(super) fn matches_source(&self, context: &Context) -> bool {
        self.matches_origin(context)
            && self.source.account_id == context.account_id
            && self.source.device_id == context.device_id
    }
    fn message(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            self.protocol,
            self.projection_id,
            self.server_origin,
            self.source.device_id,
            self.source.account_id,
            self.source.public_key,
            self.source.session_id,
            self.source.unit_id,
            self.source.revision,
            self.content.kind,
            self.content.digest,
            self.expires_at_ms,
            self.nonce,
            self.signature.algorithm,
            self.signature.key_id
        )
    }
    pub(super) fn validate(&self) -> Result<(), Error> {
        if self.protocol != PROTOCOL
            || !projection_id(&self.projection_id)
            || !origin(&self.server_origin)
            || !device(&self.source.device_id)
            || !identifier(&self.source.account_id)
            || !identifier(&self.source.session_id)
            || !identifier(&self.source.unit_id)
            || self.source.revision != 1
            || !matches!(self.content.kind.as_str(), "sticker" | "art")
            || !digest(&self.content.digest)
            || !hex32(&self.nonce)
            || !revision(self.expires_at_ms)
            || self.signature.algorithm != "ed25519"
            || self.signature.key_id != self.source.device_id
            || self.signature.value.len() != 86
            || self.source.public_key.len() != 44
        {
            return Err(Error::Envelope);
        }
        let key = STANDARD
            .decode(&self.source.public_key)
            .map_err(|_| Error::Envelope)?;
        let signature = URL_SAFE_NO_PAD
            .decode(&self.signature.value)
            .map_err(|_| Error::Envelope)?;
        if key.len() != 32
            || signature.len() != 64
            || STANDARD.encode(&key) != self.source.public_key
            || URL_SAFE_NO_PAD.encode(&signature) != self.signature.value
        {
            return Err(Error::Envelope);
        }
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
            .verify(self.message().as_bytes(), &signature)
            .map_err(|_| Error::Envelope)
    }
}
pub(super) fn device(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|value| value.hyphenated().to_string() == id)
}
fn hex32(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn projection_id(id: &str) -> bool {
    id.strip_prefix("projection:").is_some_and(hex32)
}
pub(super) fn origin(value: &str) -> bool {
    if value.len() > 256 || !value.is_ascii() || value.contains('\\') {
        return false;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    (url.scheme() == "https"
        || (url.scheme() == "http" && crate::live_relay_media_endpoint::loopback(&url)))
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && url.origin().ascii_serialization() == value
}
