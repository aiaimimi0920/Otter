//! 将固定连接交给 ureq 解析 HTTP；TLS 或可信 loopback，不创建线程、解析 DNS或复用连接。
use crate::loom_bridge_stream::BridgeStream;
use rustls::{ClientConnection, StreamOwned};
use std::{
    fmt,
    io::{Read, Write},
    net::SocketAddr,
    sync::Mutex,
};
use ureq::{
    http::Uri,
    unversioned::{
        resolver::{ResolvedSocketAddrs, Resolver},
        transport::{Buffers, ConnectionDetails, Connector, LazyBuffers, NextTimeout, Transport},
    },
};

pub(super) type TlsStream = StreamOwned<ClientConnection, BridgeStream>;
pub(super) struct Once {
    uri: Uri,
    stream: Mutex<Option<Stream>>,
}
enum Stream {
    Tls(Box<TlsStream>),
    Local(BridgeStream),
}
pub(super) struct FixedAddress(pub SocketAddr);
pub(super) struct HttpStream {
    stream: Stream,
    buffers: LazyBuffers,
}
// 库接口要求 Debug；不能输出 TLS 状态、输入缓冲或请求信息。
impl fmt::Debug for Once {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LoomHttpOnce")
    }
}
impl fmt::Debug for FixedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LoomFixedAddress")
    }
}
impl fmt::Debug for HttpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LoomHttpStream")
    }
}
impl Once {
    pub(super) fn new(uri: Uri, stream: TlsStream) -> Self {
        Self {
            uri,
            stream: Mutex::new(Some(Stream::Tls(Box::new(stream)))),
        }
    }
    pub(super) fn local(uri: Uri, stream: BridgeStream) -> Self {
        Self {
            uri,
            stream: Mutex::new(Some(Stream::Local(stream))),
        }
    }
}
impl Read for Stream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tls(s) => s.read(bytes),
            Self::Local(s) => s.read(bytes),
        }
    }
}
impl Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tls(s) => s.write(bytes),
            Self::Local(s) => s.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tls(s) => s.flush(),
            Self::Local(s) => s.flush(),
        }
    }
}
impl Resolver for FixedAddress {
    fn resolve(
        &self,
        _: &Uri,
        _: &ureq::config::Config,
        _: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        // TCP 已连接，此地址只是库所需的元数据，不发生额外解析或连接。
        let mut addresses = self.empty();
        addresses.push(self.0);
        Ok(addresses)
    }
}
impl Connector for Once {
    type Out = HttpStream;
    fn connect(
        &self,
        details: &ConnectionDetails,
        _: Option<()>,
    ) -> Result<Option<HttpStream>, ureq::Error> {
        if details.uri != &self.uri {
            return Err(ureq::Error::ConnectionFailed);
        }
        let stream = self
            .stream
            .lock()
            .map_err(|_| ureq::Error::ConnectionFailed)?
            .take()
            .ok_or(ureq::Error::ConnectionFailed)?;
        Ok(Some(HttpStream {
            stream,
            buffers: LazyBuffers::new(16 * 1024, 16 * 1024),
        }))
    }
}
impl Transport for HttpStream {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }
    fn transmit_output(&mut self, amount: usize, _: NextTimeout) -> Result<(), ureq::Error> {
        // BridgeStream 持有整个请求的绝对截止时间，不能按分片刷新。
        self.stream.write_all(&self.buffers.output()[..amount])?;
        self.stream.flush()?;
        Ok(())
    }
    fn await_input(&mut self, _: NextTimeout) -> Result<bool, ureq::Error> {
        let buffer = self.buffers.input_append_buf();
        // 解析器尚未消费满缓冲却仍要求更多输入，是容量/协议失败；零长read不能伪报远端EOF。
        if buffer.is_empty() {
            return Err(ureq::Error::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP receive buffer exhausted",
            ))));
        }
        let amount = self.stream.read(buffer)?;
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }
    fn is_open(&mut self) -> bool {
        false
    }
    fn is_tls(&self) -> bool {
        matches!(self.stream, Stream::Tls(_))
    }
}
