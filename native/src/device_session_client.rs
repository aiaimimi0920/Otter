//! 后台单次设备续签：实际 HTTPS、固定信任/地址、有界响应，不隐式注册或等待批准。
use crate::{
    device_identity::Identity,
    device_session_attempt::{self, Attempt, Session},
    loom_bridge_stream::{BridgeStream, Interrupt},
    loom_http_transport::{FixedAddress, Once},
};
use rustls::{pki_types::ServerName, ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::{
    io::Read,
    net::{SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use url::Url;
use zeroize::Zeroizing;
#[path = "device_session_failure.rs"]
mod failure;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Request,
    Origin,
    Address,
    Trust,
    Stopped,
    Deadline,
    Connect,
    Transport,
    Tls,
    Http,
    Limit,
    Clock,
    Protocol(device_session_attempt::Error),
}

/// 固定 origin 的有界 JSON 交换；仅 crate 内协议模块构造，不接受任意 URL 或任意头。
pub(super) struct Request<'a> {
    pub method: &'static str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub device: Option<(&'a str, &'a str)>,
    pub limit: usize,
}

#[derive(Clone)]
pub struct Client {
    origin: String,
    url: Url,
    addresses: Vec<SocketAddr>,
    tls: Arc<ClientConfig>,
}
impl Client {
    pub(super) fn origin(&self) -> &str {
        &self.origin
    }
    pub(super) fn media_endpoint(&self) -> crate::live_relay_media_endpoint::Endpoint {
        crate::live_relay_media_endpoint::Endpoint::from_authenticated(
            self.url.clone(),
            self.tls.clone(),
        )
    }
    pub(super) fn addresses(&self) -> Vec<SocketAddr> {
        self.addresses.clone()
    }
    /// roots 与 addresses 必须来自该 origin 的可信配置/解析 owner；不使用环境代理。
    pub fn new(
        origin: &str,
        roots: RootCertStore,
        addresses: Vec<SocketAddr>,
    ) -> Result<Self, Error> {
        let origin = device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        let url = Url::parse(&origin).map_err(|_| Error::Origin)?;
        let port = url.port_or_known_default().ok_or(Error::Address)?;
        if addresses.is_empty()
            || addresses.len() > 4
            || addresses.iter().any(|a| {
                a.port() != port
                    || (crate::live_relay_media_endpoint::loopback(&url) && !a.ip().is_loopback())
            })
        {
            return Err(Error::Address);
        }
        let tls =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|_| Error::Trust)?
                .with_root_certificates(roots)
                .with_no_client_auth();
        Ok(Self {
            origin,
            url,
            addresses,
            tls: Arc::new(tls),
        })
    }

    /// 每个 POST 在 TCP 建立后交付中断句柄，外层必须立即登记且处理已取消竞争。
    /// 只在后台调用；每个请求总期限十秒，TCP 单候选最多两秒。
    pub fn issue_once(
        &self,
        identity: &Identity,
        stop: &AtomicBool,
        connected: impl FnMut(Interrupt),
    ) -> Result<Session, Error> {
        self.issue_until(
            identity,
            stop,
            connected,
            Instant::now() + Duration::from_secs(20),
        )
    }
    pub(super) fn issue_until(
        &self,
        identity: &Identity,
        stop: &AtomicBool,
        mut connected: impl FnMut(Interrupt),
        deadline: Instant,
    ) -> Result<Session, Error> {
        let attempt = Attempt::new(identity, &self.origin).map_err(Error::Protocol)?;
        let (status, body) = self.post_until(
            device_session_attempt::CHALLENGE_PATH,
            &attempt.challenge_body(),
            stop,
            &mut connected,
            deadline.min(Instant::now() + Duration::from_secs(10)),
        )?;
        let signed = attempt.challenge(status, &body).map_err(Error::Protocol)?;
        let (status, body) = self.post_until(
            device_session_attempt::SESSION_PATH,
            signed.body(),
            stop,
            &mut connected,
            deadline.min(Instant::now() + Duration::from_secs(10)),
        )?;
        check_stop(stop)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Clock)?;
        let now = u64::try_from(now.as_millis()).map_err(|_| Error::Clock)?;
        signed.finish(status, &body, now).map_err(Error::Protocol)
    }

    pub(super) fn post(
        &self,
        path: &str,
        body: &[u8],
        stop: &AtomicBool,
        connected: &mut impl FnMut(Interrupt),
    ) -> Result<(u16, Zeroizing<Vec<u8>>), Error> {
        self.post_until(
            path,
            body,
            stop,
            connected,
            Instant::now() + Duration::from_secs(10),
        )
    }
    fn post_until(
        &self,
        path: &str,
        body: &[u8],
        stop: &AtomicBool,
        connected: &mut impl FnMut(Interrupt),
        deadline: Instant,
    ) -> Result<(u16, Zeroizing<Vec<u8>>), Error> {
        self.exchange(
            Request {
                method: "POST",
                path,
                body,
                device: None,
                limit: device_session_attempt::MAX_RESPONSE,
            },
            stop,
            connected,
            deadline,
        )
    }
    pub(super) fn exchange(
        &self,
        request: Request<'_>,
        stop: &AtomicBool,
        connected: &mut impl FnMut(Interrupt),
        deadline: Instant,
    ) -> Result<(u16, Zeroizing<Vec<u8>>), Error> {
        self.exchange_resource(request, None, stop, connected, deadline)
    }
    // 仅受验证的 Surface lease 可增加二进制预算和固定租约头；普通请求上限不变。
    pub(super) fn exchange_resource(
        &self,
        request: Request<'_>,
        resource: Option<&crate::live_relay_http::ResourceLease>,
        stop: &AtomicBool,
        connected: &mut impl FnMut(Interrupt),
        deadline: Instant,
    ) -> Result<(u16, Zeroizing<Vec<u8>>), Error> {
        check_stop(stop)?;
        let ceiling = if let Some(lease) = resource {
            if request.method != "GET"
                || request.device.is_none()
                || request.path != lease.path()
                || request.limit != lease.size()
            {
                return Err(Error::Request);
            }
            16 * 1024 * 1024
        } else if request.device.is_some()
            && request.method == "POST"
            && matches!(
                request.path,
                "/v1/walls/images/read" | "/v1/walls/surfaces/image"
            )
        {
            24 * 1024 * 1024
        } else if request.device.is_some()
            && matches!(
                (request.method, request.path),
                ("GET", "/v1/walls/state")
                    | ("POST", "/v1/walls/endpoints/register")
                    | (
                        "POST",
                        "/v1/walls/surfaces/open" | "/v1/walls/surfaces/state"
                    )
            )
        {
            10 * 1024 * 1024
        } else {
            8 * 1024 * 1024
        };
        // 仅新增专属图片提交可携带原安装预算内PNG；认证/普通POST/查询路径仍限1MiB。
        let body_ceiling = if request.method == "POST"
            && request.device.is_some()
            && matches!(
                request.path,
                "/v1/art/local/service/submit-images" | "/v1/art/local/service/submit-image-output"
            ) {
            crate::live_relay_http::art_prepare::service::MAX_IMAGE_REQUEST
        } else {
            1024 * 1024
        };
        if !matches!(request.method, "GET" | "POST")
            || !request.path.starts_with('/')
            || request.path.starts_with("//")
            || request.path.len() > 4096
            || request
                .path
                .bytes()
                .any(|c| !c.is_ascii_graphic() || c == b'#')
            || request.limit == 0
            // Surface stream 显式使用 8 MiB；其他调用点仍保留各自较小预算。
            || request.limit > ceiling
            || request.body.len() > body_ceiling
            || (request.method == "GET" && !request.body.is_empty())
        {
            return Err(Error::Request);
        }
        let uri: ureq::http::Uri = format!("{}{}", self.origin, request.path)
            .parse()
            .map_err(|_| Error::Request)?;
        let mut builder = ureq::http::Request::builder()
            .method(request.method)
            .uri(uri.clone())
            .header("content-type", "application/json")
            .header(
                "accept",
                if resource.is_some() {
                    "application/octet-stream"
                } else {
                    "application/json"
                },
            )
            .header("connection", "close");
        if let Some(lease) = resource {
            builder = builder.header("X-Loom-Surface-Lease", lease.id());
        }
        if let Some((token, nonce)) = request.device {
            if !crate::device_identity::valid_field(token, 8192)
                || nonce.len() != 32
                || !nonce
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(Error::Request);
            }
            builder = builder
                .header("authorization", format!("Device {token}"))
                .header("x-loom-device-nonce", nonce);
        }
        let wire = builder.body(request.body).map_err(|_| Error::Request)?;
        let mut chosen = None;
        for address in &self.addresses {
            check_stop(stop)?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::Deadline);
            }
            if let Ok(tcp) =
                TcpStream::connect_timeout(address, remaining.min(Duration::from_secs(2)))
            {
                chosen = Some((tcp, *address));
                break;
            }
        }
        check_stop(stop)?;
        let (tcp, address) = chosen.ok_or(Error::Connect)?;
        let mut stream = BridgeStream::new(tcp, deadline).map_err(|_| Error::Deadline)?;
        connected(stream.interrupt_handle().map_err(|_| Error::Connect)?);
        check_stop(stop)?;
        let name = self
            .url
            .host_str()
            .ok_or(Error::Origin)?
            .trim_start_matches('[')
            .trim_end_matches(']');
        let mut tls = ClientConnection::new(
            self.tls.clone(),
            ServerName::try_from(name.to_owned()).map_err(|_| Error::Origin)?,
        )
        .map_err(|_| Error::Tls)?;
        while tls.is_handshaking() {
            tls.complete_io(&mut stream).map_err(|e| {
                classify(
                    stop,
                    deadline,
                    if failure::transient_io(&e) {
                        Error::Transport
                    } else {
                        Error::Tls
                    },
                )
            })?;
        }
        let config = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .max_idle_connections(0)
            .max_response_header_size(16 * 1024)
            .build();
        let agent = ureq::Agent::with_parts(
            config,
            Once::new(uri.clone(), StreamOwned::new(tls, stream)),
            FixedAddress(address),
        );
        let mut response = agent
            .run(wire)
            .map_err(|e| classify(stop, deadline, failure::http(&e)))?;
        let status = response.status().as_u16();
        // Device 401 本身已足够撤销使用权，不能让超大/截断错误体遮蔽认证终态。
        if request.device.is_some() && status == 401 {
            check_stop(stop)?;
            return Ok((status, Zeroizing::new(Vec::new())));
        }
        if response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .is_some_and(|n| n > request.limit as u64)
        {
            return Err(Error::Limit);
        }
        // take 限制无 Content-Length/chunked 响应；读取缓冲在离开协议调用后清零。
        let mut output = Zeroizing::new(Vec::new());
        response
            .body_mut()
            .as_reader()
            .take(request.limit as u64 + 1)
            .read_to_end(&mut output)
            .map_err(|e| classify(stop, deadline, failure::body(&e)))?;
        if output.len() > request.limit {
            return Err(Error::Limit);
        }
        check_stop(stop)?;
        if Instant::now() >= deadline {
            return Err(Error::Deadline);
        }
        Ok((status, output))
    }
}
fn check_stop(stop: &AtomicBool) -> Result<(), Error> {
    if stop.load(Ordering::Acquire) {
        Err(Error::Stopped)
    } else {
        Ok(())
    }
}
fn classify(stop: &AtomicBool, deadline: Instant, fallback: Error) -> Error {
    if stop.load(Ordering::Acquire) {
        Error::Stopped
    } else if Instant::now() >= deadline {
        Error::Deadline
    } else {
        fallback
    }
}

#[cfg(test)]
#[path = "device_session_client_tests.rs"]
pub(crate) mod tests;
