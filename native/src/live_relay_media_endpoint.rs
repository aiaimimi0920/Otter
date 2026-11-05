//! 固定 Loom origin 的媒体请求与 TLS 信任；凭据获取/续签由授权 owner 负责。
use crate::live_relay_receive::MEDIA_OFFER;
use rustls::{ClientConfig, RootCertStore};
use std::{net::IpAddr, sync::Arc};
use tungstenite::{
    client::IntoClientRequest,
    http::{HeaderValue, Request},
};
use url::Url;
#[cfg(windows)]
#[path = "tile_live_endpoint.rs"]
mod wall;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Origin,
    Identity,
    Credential,
    Random,
    Request,
    Trust,
}

/// 不实现 Debug，避免调用者意外记录 token。
pub enum Credential<'a> {
    None,
    Bearer(&'a str),
    Device(&'a str),
}

pub struct Endpoint {
    origin: Url,
    tls: Option<Arc<ClientConfig>>,
}

pub struct Prepared {
    pub(super) session_id: String,
    pub(super) device_id: String,
    pub(super) request: Request<()>,
    pub(super) url: Url,
    pub(super) tls: Option<Arc<ClientConfig>>,
    pub(super) device: bool,
}
impl Prepared {
    /// 解码链已失败时重连不重新广告视频能力；只影响这份已授权Viewer请求。
    pub(super) fn viewer_legacy(&mut self) {
        self.request.headers_mut().insert(
            "sec-websocket-protocol",
            HeaderValue::from_static("loom.live.v1"),
        );
    }
    /// 仅在已建立系统encoder的Source owner内调用；不广告JPEG或改认证/URL。
    pub(crate) fn source_h264(&mut self) {
        self.request.headers_mut().insert(
            "sec-websocket-protocol",
            HeaderValue::from_static("loom.live.h264.v1,loom.live.v1"),
        );
    }
}

impl Endpoint {
    /// 仅接收已通过固定 HTTPS origin 校验的授权 Client 的 URL/信任配置。
    pub(super) fn from_authenticated(origin: Url, tls: Arc<ClientConfig>) -> Self {
        Self {
            origin,
            tls: Some(tls),
        }
    }
    /// roots 只能来自此 origin 的可信配置；不修改系统证书库，也不关闭证书验证。
    /// 明文仅允许 loopback IP 或 localhost；transport 必须再次限制解析地址。
    pub fn new(origin: &str, roots: RootCertStore) -> Result<Self, Error> {
        if origin.len() > 2048 || origin.bytes().any(|b| b.is_ascii_whitespace()) {
            return Err(Error::Origin);
        }
        let url = Url::parse(origin).map_err(|_| Error::Origin)?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
        {
            return Err(Error::Origin);
        }
        let tls = match url.scheme() {
            "https" => {
                let config = ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .map_err(|_| Error::Trust)?
                .with_root_certificates(roots)
                .with_no_client_auth();
                Some(Arc::new(config))
            }
            "http" if loopback(&url) => None,
            _ => return Err(Error::Origin),
        };
        Ok(Self { origin: url, tls })
    }

    pub fn viewer(
        &self,
        session: &str,
        device_id: &str,
        after: (u64, u64),
        credential: Credential<'_>,
    ) -> Result<Prepared, Error> {
        self.prepare(session, device_id, Some(after), credential)
    }
    pub(crate) fn source(
        &self,
        session: &str,
        device_id: &str,
        credential: Credential<'_>,
    ) -> Result<Prepared, Error> {
        self.prepare(session, device_id, None, credential)
    }
    fn prepare(
        &self,
        session: &str,
        device_id: &str,
        after: Option<(u64, u64)>,
        credential: Credential<'_>,
    ) -> Result<Prepared, Error> {
        if !crate::live_relay_viewer_identity::identifier(session)
            || !crate::live_relay_viewer_identity::identifier(device_id)
        {
            return Err(Error::Identity);
        }
        // 远端匿名和明文 Device 均不能伪装为已授权媒体连接。
        if (!loopback(&self.origin) && matches!(credential, Credential::None))
            || (self.tls.is_none() && matches!(credential, Credential::Device(_)))
        {
            return Err(Error::Credential);
        }
        let mut url = self.origin.clone();
        url.set_scheme(if self.tls.is_some() { "wss" } else { "ws" })
            .map_err(|_| Error::Origin)?;
        url.set_path("/v1/live/media");
        url.query_pairs_mut()
            .append_pair("sessionId", session)
            .append_pair("role", if after.is_some() { "viewer" } else { "source" });
        if let Some(after) = after {
            url.query_pairs_mut()
                .append_pair("afterEpoch", &after.0.to_string())
                .append_pair("afterFrameId", &after.1.to_string());
        }
        // Device identity comes exclusively from the authenticated session. Keep
        // the query identity only for the existing local/admin compatibility path.
        if !matches!(credential, Credential::Device(_)) {
            url.query_pairs_mut().append_pair("deviceId", device_id);
        }
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|_| Error::Request)?;
        request.headers_mut().insert(
            "sec-websocket-protocol",
            HeaderValue::from_static(if after.is_some() {
                MEDIA_OFFER
            } else {
                "loom.live.v1"
            }),
        );
        let (scheme, token) = match credential {
            Credential::None => ("", ""),
            Credential::Bearer(token) => ("Bearer", token),
            Credential::Device(token) => ("Device", token),
        };
        if !scheme.is_empty() {
            if token.is_empty()
                || token.len() > 8192
                || !token.bytes().all(|b| b.is_ascii_graphic())
            {
                return Err(Error::Credential);
            }
            let mut header = HeaderValue::from_str(&format!("{scheme} {token}"))
                .map_err(|_| Error::Credential)?;
            header.set_sensitive(true);
            request.headers_mut().insert("authorization", header);
        }
        let device = scheme == "Device";
        if device {
            use base64::Engine;
            let mut bytes = [0u8; 24];
            rustls::crypto::ring::default_provider()
                .secure_random
                .fill(&mut bytes)
                .map_err(|_| Error::Random)?;
            let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
            request.headers_mut().insert(
                "x-loom-device-nonce",
                HeaderValue::from_str(&nonce).map_err(|_| Error::Request)?,
            );
        }
        Ok(Prepared {
            session_id: session.to_owned(),
            device_id: device_id.to_owned(),
            request,
            url,
            tls: self.tls.clone(),
            device,
        })
    }
}

pub(super) fn loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => IpAddr::V4(ip).is_loopback(),
        Some(url::Host::Ipv6(ip)) => IpAddr::V6(ip).is_loopback(),
        Some(url::Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
        _ => false,
    }
}

#[cfg(test)]
#[path = "live_relay_media_endpoint_tests.rs"]
mod tests;
