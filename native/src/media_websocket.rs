//! 已认证媒体的TCP/TLS/Upgrade窄接缝；不解释Viewer、Source或Wall业务协议。
use crate::live_relay_media_transport::Error;
use crate::{
    live_relay_media_endpoint::Prepared,
    loom_bridge_stream::{BridgeStream, Interrupt},
};
use rustls::{pki_types::ServerName, ClientConnection, StreamOwned};
use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tungstenite::{protocol::WebSocketConfig, WebSocket};

pub(crate) enum Stream {
    Plain(BridgeStream),
    Tls(Box<StreamOwned<ClientConnection, BridgeStream>>),
}
impl Stream {
    pub(crate) fn base(&mut self) -> &mut BridgeStream {
        match self {
            Self::Plain(s) => s,
            Self::Tls(s) => &mut s.sock,
        }
    }
}
impl Read for Stream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(bytes),
            Self::Tls(s) => s.read(bytes),
        }
    }
}
impl Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(bytes),
            Self::Tls(s) => s.write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}
pub(crate) struct Upgraded {
    pub socket: WebSocket<Stream>,
    pub protocol: String,
    pub device: bool,
}
/// 最多四个固定地址，每候选2秒；TLS和upgrade共用3秒，不改变信任身份或系统证书。
pub(crate) fn connect(
    prepared: Prepared,
    addresses: &[SocketAddr],
    stop: &AtomicBool,
    connected: impl FnOnce(Interrupt),
    config: WebSocketConfig,
) -> Result<Upgraded, Error> {
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    if addresses.is_empty() || addresses.len() > 4 {
        return Err(Error::Address);
    }
    let port = prepared.url.port_or_known_default().ok_or(Error::Address)?;
    if addresses.iter().any(|a| {
        a.port() != port
            || (crate::live_relay_media_endpoint::loopback(&prepared.url) && !a.ip().is_loopback())
    }) {
        return Err(Error::Address);
    }
    let mut tcp = None;
    for address in addresses {
        if stop.load(Ordering::Acquire) {
            return Err(Error::Stopped);
        }
        if let Ok(stream) = TcpStream::connect_timeout(address, Duration::from_secs(2)) {
            tcp = Some(stream);
            break;
        }
    }
    let mut base = BridgeStream::new(
        tcp.ok_or(Error::Connect)?,
        Instant::now() + Duration::from_secs(3),
    )
    .map_err(|_| Error::Deadline)?;
    connected(base.interrupt_handle().map_err(|_| Error::Connect)?);
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    let stream = if let Some(config) = prepared.tls {
        let name = prepared
            .url
            .host_str()
            .ok_or(Error::Address)?
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let mut tls =
            ClientConnection::new(config, ServerName::try_from(name).map_err(|_| Error::Tls)?)
                .map_err(|_| Error::Tls)?;
        while tls.is_handshaking() {
            tls.complete_io(&mut base).map_err(|_| Error::Tls)?;
        }
        Stream::Tls(Box::new(StreamOwned::new(tls, base)))
    } else {
        Stream::Plain(base)
    };
    let offered = prepared
        .request
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::Profile)?
        .to_owned();
    let (socket, response) =
        tungstenite::client::client_with_config(prepared.request, stream, Some(config))
            .map_err(|_| Error::Upgrade)?;
    let mut protocols = response.headers().get_all("sec-websocket-protocol").iter();
    let selected = protocols
        .next()
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::Profile)?;
    if protocols.next().is_some() || !offered.split(',').any(|p| p.trim() == selected) {
        return Err(Error::Profile);
    }
    if stop.load(Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    Ok(Upgraded {
        socket,
        protocol: selected.to_owned(),
        device: prepared.device,
    })
}
