//! 签名源码执行许可的客户端准入与不可逆终止；不自行建立 issuer 信任，也不启动 JS。
//! 包签名、资源 lease 和 Device token 均不能替代此合同；网络与文件接口不因许可自动开放。
#[path = "art_execution_package.rs"]
pub(crate) mod package;
#[path = "art_execution_resume.rs"]
pub(crate) mod resume;
#[path = "art_execution_grant_wire.rs"]
pub mod wire;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::signature::{UnparsedPublicKey, ED25519};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use wire::{Binding, Envelope, Grant, Permission};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Wire,
    Signature,
    Binding,
    Scope,
    Time,
    Source,
    Stopped,
}

/// 必须由已认证的配置/握手提供；响应自带的 key 不得成为 pin。
pub struct Issuer<'a> {
    pub key_id: &'a str,
    pub public_key: &'a [u8; 32],
}
pub struct Expected<'a> {
    pub binding: &'a Binding,
    pub permissions: &'a [Permission],
    pub services: &'a [String],
}
pub struct Permit {
    grant: Grant,
    started: Instant,
    remaining: Duration,
    offline_remaining: Option<Duration>,
    last_wall: u64,
    stopped: bool,
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn signed_payload(bytes: &[u8], issuer: Issuer<'_>, domain: &[u8]) -> Result<Vec<u8>, Error> {
    if bytes.len() > wire::MAX_ENVELOPE {
        return Err(Error::Limit);
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
    if envelope.algorithm != "ed25519" || envelope.key_id != issuer.key_id {
        return Err(Error::Signature);
    }
    let payload = BASE64.decode(envelope.payload).map_err(|_| Error::Wire)?;
    if payload.len() > wire::MAX_PAYLOAD {
        return Err(Error::Limit);
    }
    let signature = BASE64
        .decode(envelope.signature)
        .map_err(|_| Error::Signature)?;
    UnparsedPublicKey::new(&ED25519, issuer.public_key)
        .verify(&[domain, payload.as_slice()].concat(), &signature)
        .map_err(|_| Error::Signature)?;
    Ok(payload)
}
fn package_component(value: &str) -> bool {
    let base = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && !["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
        && !["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix)
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=9).contains(&n))
        })
}
impl Permit {
    pub(crate) fn grant_id(&self) -> &str {
        &self.grant.grant_id
    }
    /// 完整签名、精确请求身份、授权范围和下载源码全部通过后才产生 permit。
    /// 原子缓存/持久化时钟高水位、issuer 配对和生产挂载仍由上层 owner 接线。
    pub fn accept(
        bytes: &[u8],
        issuer: Issuer<'_>,
        expected: Expected<'_>,
        source: &[u8],
        now: u64,
    ) -> Result<Self, Error> {
        if bytes.len() > wire::MAX_ENVELOPE || source.len() > 1024 * 1024 {
            return Err(Error::Limit);
        }
        let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
        if envelope.algorithm != "ed25519" || envelope.key_id != issuer.key_id {
            return Err(Error::Signature);
        }
        let payload = BASE64.decode(envelope.payload).map_err(|_| Error::Wire)?;
        if payload.len() > wire::MAX_PAYLOAD {
            return Err(Error::Limit);
        }
        let signature = BASE64
            .decode(envelope.signature)
            .map_err(|_| Error::Signature)?;
        UnparsedPublicKey::new(&ED25519, issuer.public_key)
            .verify(&[wire::DOMAIN, payload.as_slice()].concat(), &signature)
            .map_err(|_| Error::Signature)?;
        let grant: Grant = serde_json::from_slice(&payload).map_err(|_| Error::Wire)?;
        validate(&grant, &expected, source, now)?;
        Ok(Self::from_grant(grant, now))
    }
    fn from_grant(grant: Grant, now: u64) -> Self {
        let offline_remaining = grant
            .offline_until_ms
            .checked_sub(now)
            .filter(|n| *n > 0)
            .map(Duration::from_millis);
        Self {
            remaining: Duration::from_millis(grant.expires_at_ms - now),
            offline_remaining,
            grant,
            started: Instant::now(),
            last_wall: now,
            stopped: false,
        }
    }
    pub fn stop(&mut self) {
        self.stopped = true;
    }
    /// 仅安排等待，不能替代 authorize；沿用首次准入的单调截止，不能从挂载时重新计时。
    pub(crate) fn deadline(&self, online: bool) -> (u64, Instant) {
        let remaining = if online {
            self.remaining
        } else {
            self.remaining
                .min(self.offline_remaining.unwrap_or_default())
        };
        let wall = if online {
            self.grant.expires_at_ms
        } else {
            self.grant.expires_at_ms.min(self.grant.offline_until_ms)
        };
        (wall, self.started + remaining)
    }
    pub(crate) fn observed_wall(&self) -> u64 {
        self.last_wall
    }
    /// 仅撤销已提交的服务；允许本地执行已停止，但绝不恢复执行许可或返回 Grant。
    pub(crate) fn authorize_cancel(&mut self, current: &Binding, now: u64) -> Result<(), Error> {
        if current != &self.grant.binding {
            return Err(Error::Binding);
        }
        if now < self.last_wall
            || now >= self.grant.expires_at_ms
            || self.started.elapsed() >= self.remaining
        {
            self.stop();
            return Err(Error::Time);
        }
        self.last_wall = now;
        Ok(())
    }
    pub fn authorize(
        &mut self,
        current: &Binding,
        now: u64,
        online: bool,
        revoked: bool,
    ) -> Result<&Grant, Error> {
        self.check(current, now, online, revoked, self.started.elapsed())?;
        Ok(&self.grant)
    }
    /// UI宿主资格只读检查，不推进worker共享时钟；否则可能误判已取样的服务请求发生时钟回退。
    pub(crate) fn validate_online_host(&self, current: &Binding, now: u64) -> Result<(), Error> {
        self.invalid(current, now, true, false, self.started.elapsed())
            .map_or(Ok(()), Err)
    }
    fn check(
        &mut self,
        current: &Binding,
        now: u64,
        online: bool,
        revoked: bool,
        elapsed: Duration,
    ) -> Result<(), Error> {
        if let Some(error) = self.invalid(current, now, online, revoked, elapsed) {
            self.stop(); // 过期/撤权/换实例/时钟回退后不能被晚到结果复活。
            return Err(error);
        }
        self.last_wall = now;
        Ok(())
    }
    fn invalid(
        &self,
        current: &Binding,
        now: u64,
        online: bool,
        revoked: bool,
        elapsed: Duration,
    ) -> Option<Error> {
        if self.stopped {
            return Some(Error::Stopped);
        }
        if revoked {
            Some(Error::Stopped)
        } else if current != &self.grant.binding {
            Some(Error::Binding)
        } else if now < self.last_wall
            || now >= self.grant.expires_at_ms
            || elapsed >= self.remaining
            || (!online
                && (self.grant.offline_until_ms == 0
                    || now >= self.grant.offline_until_ms
                    || self.offline_remaining.is_none_or(|limit| elapsed >= limit)))
        {
            Some(Error::Time)
        } else {
            None
        }
    }
}

/// 描述消费和签名准入共享请求约束；此校验本身不产生执行许可。
pub(crate) fn validate_expected(expected: &Expected<'_>) -> Result<(), Error> {
    let b = expected.binding;
    if b.sdk_version != wire::SDK
        || b.generation == 0
        || b.generation > 9_007_199_254_740_991
        || [&b.request_id, &b.device_id, &b.workspace_id, &b.instance_id]
            .iter()
            .any(|v| !identifier(v))
        || !b
            .art_id
            .split_once('/')
            .is_some_and(|(publisher, art)| package_component(publisher) && package_component(art))
        || !digest(&b.package_digest)
        || b.origin.is_empty()
        || b.origin.len() > 2048
        || !b.origin.is_ascii()
        || b.origin
            .bytes()
            .any(|c| c.is_ascii_control() || c.is_ascii_whitespace())
        || b.art_version.is_empty()
        || b.art_version.len() > 128
        || !b
            .art_version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".+-".contains(&c))
    {
        return Err(Error::Binding);
    }
    if expected.permissions.is_empty()
        || expected.permissions.len() > 4
        || expected
            .permissions
            .iter()
            .enumerate()
            .any(|(i, p)| expected.permissions[..i].contains(p))
        || expected.services.len() > 32
        || expected
            .services
            .iter()
            .enumerate()
            .any(|(i, s)| !identifier(s) || expected.services[..i].contains(s))
        || (expected.services.is_empty() == expected.permissions.contains(&Permission::LoomService))
    {
        return Err(Error::Scope);
    }
    Ok(())
}
fn validate(grant: &Grant, expected: &Expected<'_>, source: &[u8], now: u64) -> Result<(), Error> {
    if grant.protocol_version != wire::PROTOCOL
        || grant.execution_owner != "otter"
        || grant.result_owner != "otter_workspace"
        || &grant.binding != expected.binding
        || !identifier(&grant.grant_id)
    {
        return Err(Error::Binding);
    }
    if grant.permissions != expected.permissions || grant.services != expected.services {
        return Err(Error::Scope);
    }
    validate_expected(expected)?;
    if grant.issued_at_ms == 0
        || now < grant.issued_at_ms
        || now >= grant.expires_at_ms
        || grant.expires_at_ms > 9_007_199_254_740_991
        || grant.expires_at_ms <= grant.issued_at_ms
        || grant.expires_at_ms - grant.issued_at_ms > 15 * 60 * 1000
        || (grant.offline_until_ms != 0
            && (grant.offline_until_ms <= grant.issued_at_ms
                || grant.offline_until_ms > grant.expires_at_ms))
    {
        return Err(Error::Time);
    }
    let hash = Sha256::digest(source)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if source.is_empty()
        || source.contains(&0)
        || std::str::from_utf8(source).is_err()
        || grant.source.size != source.len() as u64
        || grant.source.digest != hash
        || grant.source.resource_id != format!("sha256:{hash}")
    {
        return Err(Error::Source);
    }
    Ok(())
}

#[cfg(test)]
#[path = "art_execution_grant_tests.rs"]
mod tests;
