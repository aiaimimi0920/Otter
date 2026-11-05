//! 可信发现数据的身份校验；输入仍须由发现层验证来源，本模块不授权任意端点。
use base64::Engine;
use sha2::{Digest, Sha256};
use std::{
    fmt,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

/// 目录、准备和执行共享同一固定身份；克隆只增加引用计数，不重新发现或复制凭据。
#[derive(Clone)]
pub struct Identity {
    inner: Arc<Verified>,
}
struct Verified {
    endpoint: String,
    instance: String,
    token: String,
    certificate: Vec<u8>,
    address: SocketAddr,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Protocol,
    Instance,
    Credential,
    Endpoint,
    Certificate,
    Pin,
}

impl fmt::Debug for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Identity { [native credentials redacted] }")
    }
}

impl Identity {
    pub fn validate(
        protocol: &str,
        instance: &str,
        endpoint: &str,
        certificate_base64: &str,
        certificate_sha256: &str,
        token: &str,
    ) -> Result<Self, Error> {
        if protocol != "loom.local-bridge.v1" {
            return Err(Error::Protocol);
        }
        if instance.len() > 64 || uuid::Uuid::parse_str(instance).is_err() {
            return Err(Error::Instance);
        }
        if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::Credential);
        }
        let port = endpoint
            .strip_prefix("wss://127.0.0.1:")
            .and_then(|value| value.strip_suffix('/'))
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|port| *port != 0)
            .ok_or(Error::Endpoint)?;
        if endpoint != format!("wss://127.0.0.1:{port}/") {
            return Err(Error::Endpoint);
        }
        if certificate_base64.is_empty() || certificate_base64.len() > 8192 {
            return Err(Error::Certificate);
        }
        let certificate = base64::engine::general_purpose::STANDARD
            .decode(certificate_base64)
            .map_err(|_| Error::Certificate)?;
        if certificate.is_empty() {
            return Err(Error::Certificate);
        }
        let digest = format!("{:x}", Sha256::digest(&certificate));
        if certificate_sha256 != digest {
            return Err(Error::Pin);
        }
        Ok(Self {
            inner: Arc::new(Verified {
                endpoint: endpoint.into(),
                instance: instance.into(),
                token: token.into(),
                certificate,
                address: (Ipv4Addr::LOCALHOST, port).into(),
            }),
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.inner.address
    }
    pub fn endpoint(&self) -> &str {
        &self.inner.endpoint
    }
    pub fn instance(&self) -> &str {
        &self.inner.instance
    }
    pub fn certificate(&self) -> &[u8] {
        &self.inner.certificate
    }
    /// 仅交给 TLS exporter；不能作为 HTTP header 或日志内容。
    pub fn exporter_context(&self) -> String {
        format!("{}:{}", self.inner.instance, self.inner.token)
    }
    /// 环境覆盖只能重复已验证地址，不能替换发现提供的信任边界。
    pub fn check_endpoint_override(&self, value: Option<&str>) -> Result<(), Error> {
        if value
            .is_some_and(|value| !value.trim().is_empty() && value.trim() != self.inner.endpoint)
        {
            Err(Error::Endpoint)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "loom_bridge_identity_tests.rs"]
mod tests;
