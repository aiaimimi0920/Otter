//! 客户端独立包证明DTO/验签；绑定同次完整grant和有界资源，路径从不进入本地执行设施。
use super::*;
use serde::{Deserialize, Serialize};
#[path = "art_execution_ports.rs"]
pub(crate) mod ports;
#[path = "art_execution_visibility.rs"]
pub(crate) mod visibility;
pub(crate) const PROTOCOL: &str = "loom.art.local.package.v1";
#[cfg(test)]
pub(crate) const DOMAIN: &[u8] = b"loom.art.local.package.v1\0";
#[cfg(test)]
pub(crate) const PREPARE: &str = "loom.art.local.prepare-package.v1";
pub(crate) const RESOURCE: &str = "loom.art.local.resource.v1";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Resource {
    pub id: String,
    pub mime_type: String,
    pub size: u64,
    pub digest: String,
    pub dimensions: [u32; 2],
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Package {
    pub protocol_version: String,
    pub grant: Grant,
    pub device_public_key: String,
    pub resources: Vec<Resource>,
}
pub(crate) fn validate_resources(resources: &[Resource]) -> Result<(), Error> {
    if resources.len() > 16 {
        return Err(Error::Limit);
    }
    let (mut png, mut pixels, mut inline, mut images) = (0u64, 0u64, 2u64, 0);
    for (i, r) in resources.iter().enumerate() {
        if r.id.is_empty()
            || r.id.len() > 96
            || !r
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || resources[..i].iter().any(|v| v.id == r.id)
            || !digest(&r.digest)
            || r.size == 0
        {
            return Err(Error::Source);
        }
        if r.mime_type == "image/png" {
            let [w, h] = r.dimensions;
            if w == 0 || h == 0 || r.size > 4 * 1024 * 1024 {
                return Err(Error::Limit);
            }
            pixels = u64::from(w)
                .checked_mul(u64::from(h))
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| pixels.checked_add(n))
                .ok_or(Error::Limit)?;
            png += r.size;
            images += 1;
        } else {
            if !matches!(
                r.mime_type.as_str(),
                "text/plain" | "application/json" | "application/octet-stream"
            ) || r.dimensions != [0, 0]
                || r.size > 256 * 1024
            {
                return Err(Error::Limit);
            }
            inline += 4 * r.size.div_ceil(3) + r.id.len() as u64 + r.mime_type.len() as u64 + 64;
        }
        if images > 8 || png > 8 * 1024 * 1024 || pixels > 16 * 1024 * 1024 || inline > 512 * 1024 {
            return Err(Error::Limit);
        }
    }
    Ok(())
}
impl Package {
    #[cfg(test)]
    pub(crate) fn verify(bytes: &[u8], issuer: Issuer<'_>) -> Result<Self, Error> {
        let payload = signed_payload(bytes, issuer, DOMAIN)?;
        let package: Self = serde_json::from_slice(&payload).map_err(|_| Error::Wire)?;
        package.validate()?;
        Ok(package)
    }
    pub(super) fn validate(&self) -> Result<(), Error> {
        if self.protocol_version != PROTOCOL {
            return Err(Error::Wire);
        }
        if !self.device_public_key.is_empty() {
            let key = BASE64
                .decode(&self.device_public_key)
                .map_err(|_| Error::Binding)?;
            if key.len() != 32
                || BASE64.encode(key) != self.device_public_key
                || self.grant.offline_until_ms == 0
            {
                return Err(Error::Binding);
            }
        }
        validate_resources(&self.resources)
    }
}
