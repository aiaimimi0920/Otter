//! 当前已授权 Loom 的固定 TLS 传输；测试 loopback 共用取消、期限及有界响应合同。
use super::*;
use crate::{
    loom_bridge_stream::BridgeStream,
    loom_http_transport::{FixedAddress, Once},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::rand::{SecureRandom, SystemRandom};
use std::{
    io::Read,
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
pub(super) struct Endpoint {
    origin: String,
    address: SocketAddr,
    tls: Option<crate::device_session_client::Client>,
}
pub(super) struct Bytes {
    data: Zeroizing<Vec<u8>>,
    _budget: crate::byte_budget::Lease,
}
impl std::ops::Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.data
    }
}
impl Endpoint {
    #[cfg(test)]
    pub(super) fn new(target: crate::loom_media_target::Target) -> Result<Self, Error> {
        let (origin, _) = target.local_control().map_err(|_| Error::LocalOnly)?;
        let mut url = url::Url::parse(origin).map_err(|_| Error::LocalOnly)?;
        if url.host_str() == Some("localhost") {
            url.set_host(Some("127.0.0.1"))
                .map_err(|_| Error::LocalOnly)?;
        }
        let ip: std::net::IpAddr = url
            .host_str()
            .ok_or(Error::LocalOnly)?
            .trim_matches(['[', ']'])
            .parse()
            .map_err(|_| Error::LocalOnly)?;
        if !ip.is_loopback()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(Error::LocalOnly);
        }
        Ok(Self {
            origin: url.origin().ascii_serialization(),
            address: SocketAddr::new(ip, url.port_or_known_default().ok_or(Error::LocalOnly)?),
            tls: None,
        })
    }
    pub(super) fn authorized(client: crate::device_session_client::Client) -> Self {
        Self {
            origin: client.origin().into(),
            address: client.addresses()[0],
            tls: Some(client),
        }
    }
    pub(super) fn authenticates(&self, origin: &str) -> bool {
        self.tls.is_some() && self.origin == origin
    }
    pub(super) fn request(
        &self,
        method: &'static str,
        path: &str,
        body: &[u8],
        session: Option<&Session>,
        cancel: &Cancel,
        limit: usize,
    ) -> Result<Bytes, Error> {
        check(cancel)?;
        if limit == 0
            || limit > MAX_WIRE
            || body.len() > MAX_WIRE
            || (method == "GET" && !body.is_empty())
            || !matches!(
                (method, path, session.is_some()),
                ("GET", "/v1/otter/identity", false)
                    | (
                        "POST",
                        "/v1/projections/v2/context"
                            | "/v1/projections/v2/create"
                            | "/v1/projections/v2/inspect"
                            | "/v1/projections/v2/accept"
                            | "/v1/projections/v2/update"
                            | "/v1/projections/v2/read"
                            | "/v1/projections/v2/unlink",
                        true
                    )
            )
        {
            return Err(Error::Input);
        }
        let mut budget = crate::pixels::reserve(limit.next_power_of_two()).ok_or(Error::Budget)?;
        if let Some(client) = &self.tls {
            let time = now()?;
            let token = session
                .map(|s| s.token_for(client.origin(), time))
                .transpose()
                .map_err(|_| Error::Credential)?;
            let mut nonce = [0u8; 24];
            SystemRandom::new()
                .fill(&mut nonce)
                .map_err(|_| Error::Http)?;
            let nonce = URL_SAFE_NO_PAD.encode(nonce);
            let result = client.exchange(
                crate::device_session_client::Request {
                    method,
                    path,
                    body,
                    device: token.map(|token| (token, nonce.as_str())),
                    limit,
                },
                cancel.flag(),
                &mut |handle| cancel.install(handle),
                Instant::now() + Duration::from_secs(20),
            );
            cancel.clear();
            let (status, data) = result.map_err(|_| failure(cancel))?;
            check(cancel)?;
            if !(200..300).contains(&status) {
                let value = serde_json::from_slice::<serde_json::Value>(&data).ok();
                let code = value
                    .as_ref()
                    .and_then(|v| v["error"]["code"].as_str())
                    .filter(|s| identifier(s))
                    .map(str::to_owned);
                return Err(Error::Remote { status, code });
            }
            if let Some(session) = session {
                session
                    .token_for(client.origin(), now()?)
                    .map_err(|_| Error::Credential)?;
            }
            if !budget.grow_to(data.capacity()) {
                return Err(Error::Budget);
            }
            return Ok(Bytes {
                data,
                _budget: budget,
            });
        }
        let uri: ureq::http::Uri = format!("{}{path}", self.origin)
            .parse()
            .map_err(|_| Error::LocalOnly)?;
        let mut request = ureq::http::Request::builder()
            .method(method)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .header("connection", "close");
        if let Some(session) = session {
            let token = session
                .token_for(session.origin(), now()?)
                .map_err(|_| Error::Credential)?;
            let mut nonce = [0u8; 24];
            SystemRandom::new()
                .fill(&mut nonce)
                .map_err(|_| Error::Http)?;
            let auth = Zeroizing::new(format!("Device {token}"));
            request = request
                .header("authorization", &*auth)
                .header("x-loom-device-nonce", URL_SAFE_NO_PAD.encode(nonce));
        }
        let request = request.body(body).map_err(|_| Error::Input)?;
        let deadline = Instant::now() + Duration::from_secs(20);
        let tcp = TcpStream::connect_timeout(&self.address, Duration::from_secs(2))
            .map_err(|_| failure(cancel))?;
        let stream = BridgeStream::new(tcp, deadline).map_err(|_| failure(cancel))?;
        cancel.install(stream.interrupt_handle().map_err(|_| Error::Http)?);
        struct Clear<'a>(&'a Cancel);
        impl Drop for Clear<'_> {
            fn drop(&mut self) {
                self.0.clear();
            }
        }
        let _clear = Clear(cancel);
        check(cancel)?;
        let config = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .max_idle_connections(0)
            .max_response_header_size(16 * 1024)
            .build();
        let agent =
            ureq::Agent::with_parts(config, Once::local(uri, stream), FixedAddress(self.address));
        let mut response = agent.run(request).map_err(|_| failure(cancel))?;
        let status = response.status().as_u16();
        let read = (|| {
            if response
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .is_some_and(|n| n > limit as u64)
            {
                return Err(Error::Budget);
            }
            let mut bytes = Zeroizing::new(Vec::new());
            response
                .body_mut()
                .as_reader()
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| failure(cancel))?;
            if bytes.len() > limit || !budget.grow_to(bytes.capacity()) {
                return Err(Error::Budget);
            }
            Ok(bytes)
        })();
        check(cancel)?;
        // 401 的撤权事实不能被超大或截断错误体遮蔽；可用的业务 code 仍保留。
        if !(200..300).contains(&status) {
            #[derive(Deserialize)]
            struct Failure<'a> {
                #[serde(borrow)]
                error: Code<'a>,
            }
            #[derive(Deserialize)]
            struct Code<'a> {
                code: &'a str,
            }
            let code = read
                .as_ref()
                .ok()
                .and_then(|b| serde_json::from_slice::<Failure<'_>>(b).ok())
                .and_then(|v| identifier(v.error.code).then(|| v.error.code.to_owned()));
            if status != 401 {
                if let Err(error) = read {
                    return Err(error);
                }
            }
            return Err(Error::Remote { status, code });
        }
        if let Some(session) = session {
            session
                .token_for(session.origin(), now()?)
                .map_err(|_| Error::Credential)?;
        }
        Ok(Bytes {
            data: read?,
            _budget: budget,
        })
    }
}
fn failure(cancel: &Cancel) -> Error {
    if cancel.is_stopped() {
        Error::Stopped
    } else {
        Error::Http
    }
}
