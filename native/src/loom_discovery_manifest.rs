//! 有界发现清单解析；只返回经验证的桥接身份，不实现发现路径信任或公开凭据序列化。
use crate::loom_bridge_identity::Identity;
use serde::Deserialize;
use std::{collections::HashSet, net::IpAddr};
pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    TooLarge,
    Parse,
    Metadata,
    Transport,
    Capabilities,
    MissingBridge,
    Identity,
}

// 不派生 Debug/Serialize：原始清单包含桥接令牌和 HTTP bearer 凭据。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    app_id: String,
    display_name: String,
    version: String,
    pid: Option<u32>,
    started_at: Option<serde_json::Value>,
    transport: Transport,
    #[serde(default)]
    capabilities: Vec<String>,
    hook_bridge: Option<Bridge>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transport {
    #[serde(rename = "type")]
    kind: String,
    base_url: String,
    auth: Option<String>,
    auth_token: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bridge {
    protocol: String,
    instance_id: String,
    endpoint: String,
    certificate_der_base64: String,
    certificate_sha256: String,
    auth_token: String,
}

pub fn parse(bytes: &[u8]) -> Result<Identity, Error> {
    let manifest = validated(bytes)?;
    let bridge = manifest.hook_bridge.ok_or(Error::MissingBridge)?;
    Identity::validate(
        &bridge.protocol,
        &bridge.instance_id,
        &bridge.endpoint,
        &bridge.certificate_der_base64,
        &bridge.certificate_sha256,
        &bridge.auth_token,
    )
    .map_err(|_| Error::Identity)
}

/// HTTP 媒体发现不要求 hookBridge；两条路径共享完整清单与 transport 校验。
pub fn parse_media(bytes: &[u8]) -> Result<crate::loom_media_target::Target, Error> {
    let manifest = validated(bytes)?;
    let transport = manifest.transport;
    let token = transport
        .auth
        .as_deref()
        .unwrap_or("none")
        .eq_ignore_ascii_case("bearer")
        .then(|| transport.auth_token.unwrap_or_default().trim().to_owned());
    Ok(crate::loom_media_target::Target::new(
        transport.base_url,
        token,
    ))
}

fn validated(bytes: &[u8]) -> Result<Manifest, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| Error::Parse)?;
    if manifest.schema_version != 1
        || manifest.app_id != "loom"
        || manifest.display_name.trim().is_empty()
        || manifest.version.trim().is_empty()
        || manifest.pid.is_none()
        || manifest.started_at.is_none()
    {
        return Err(Error::Metadata);
    }
    validate_transport(&manifest.transport)?;
    let mut names = HashSet::new();
    for name in &manifest.capabilities {
        if name.trim().is_empty() || !names.insert(name.as_str()) {
            return Err(Error::Capabilities);
        }
    }
    if !names.contains("brain.plan") {
        return Err(Error::Capabilities);
    }
    Ok(manifest)
}

fn validate_transport(transport: &Transport) -> Result<(), Error> {
    if !transport.kind.eq_ignore_ascii_case("http") {
        return Err(Error::Transport);
    }
    let url = url::Url::parse(&transport.base_url).map_err(|_| Error::Transport)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Transport);
    }
    let host = url.host_str().ok_or(Error::Transport)?;
    let ip = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    let loopback = host.eq_ignore_ascii_case("localhost")
        || ip.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    // 与主干默认 remote-surface 清单验证一致；桥接身份仍单独限定 IPv4 本机 WSS。
    if url.scheme() == "http" && !loopback {
        return Err(Error::Transport);
    }
    let auth = transport.auth.as_deref().unwrap_or("none");
    if !auth.eq_ignore_ascii_case("none") && !auth.eq_ignore_ascii_case("bearer") {
        return Err(Error::Transport);
    }
    if auth.eq_ignore_ascii_case("bearer")
        && transport
            .auth_token
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err(Error::Transport);
    }
    Ok(())
}

#[cfg(test)]
#[path = "loom_discovery_manifest_tests.rs"]
mod tests;
