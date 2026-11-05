//! 离线恢复只读当前 v3 签名包；恢复能力不依赖旧协议或旧程序互通。
use super::*;
use serde::{Deserialize, Serialize};
#[cfg(test)]
pub(crate) const DOMAIN: &[u8] = b"loom.art.local.resume.v1\0";
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Ticket {
    pub protocol_version: String,
    pub device_public_key: String,
    pub grant: Grant,
    #[serde(skip)]
    pub resources: Vec<super::package::Resource>,
    #[serde(skip)]
    pub ports: Option<super::package::ports::Ports>,
    #[serde(skip)]
    pub visibility: Option<super::package::visibility::Visibility>,
}
impl Ticket {
    pub(crate) fn verify(bytes: &[u8], issuer: Issuer<'_>) -> Result<Self, Error> {
        let proof = super::package::visibility::Package::verify(bytes, issuer)?;
        let package = proof.package;
        if package.device_public_key.is_empty() {
            return Err(Error::Binding);
        }
        Ok(Self {
            protocol_version: super::package::visibility::PROTOCOL.into(),
            device_public_key: package.device_public_key,
            grant: package.grant,
            resources: package.resources,
            ports: Some(proof.ports),
            visibility: Some(proof.visibility),
        })
    }
    pub(crate) fn permit(
        &self,
        expected: Expected<'_>,
        source: &[u8],
        now: u64,
    ) -> Result<Permit, Error> {
        let old = &self.grant.binding;
        let new = expected.binding;
        if (
            &old.origin,
            &old.device_id,
            &old.art_id,
            &old.art_version,
            &old.package_digest,
            &old.sdk_version,
        ) != (
            &new.origin,
            &new.device_id,
            &new.art_id,
            &new.art_version,
            &new.package_digest,
            &new.sdk_version,
        ) {
            return Err(Error::Binding);
        }
        // 先校原签名内的完整grant，再仅以显式resume权限创建内存中的当前绑定；绝不签发/输出新v1凭据。
        validate(
            &self.grant,
            &Expected {
                binding: old,
                permissions: expected.permissions,
                services: expected.services,
            },
            source,
            now,
        )?;
        validate_expected(&expected)?;
        if now >= self.grant.offline_until_ms {
            return Err(Error::Time);
        }
        let mut grant = self.grant.clone();
        grant.binding = new.clone();
        grant.expires_at_ms = grant.expires_at_ms.min(grant.offline_until_ms);
        Ok(Permit::from_grant(grant, now))
    }
}
impl Permit {
    pub(crate) fn accept_resumed(
        bytes: &[u8],
        issuer: Issuer<'_>,
        expected: Expected<'_>,
        source: &[u8],
        now: u64,
    ) -> Result<Self, Error> {
        Ticket::verify(bytes, issuer)?.permit(expected, source, now)
    }
}
