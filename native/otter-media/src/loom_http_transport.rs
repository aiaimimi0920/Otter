//! Hand a fixed connection to ureq for HTTP parsing; TLS or trusted loopback, without creating threads, resolving DNS, or reusing connections.
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

pub type TlsStream = StreamOwned<ClientConnection, BridgeStream>;
pub struct Once {
    uri: Uri,
    stream: Mutex<Option<Stream>>,
}
enum Stream {
    Tls(Box<TlsStream>),
    Local(BridgeStream),
}
pub struct FixedAddress(pub SocketAddr);
pub struct HttpStream {
    stream: Stream,
    buffers: LazyBuffers,
}
// The library interface requires Debug; do not expose TLS state, input buffers, or request information.
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
    pub fn new(uri: Uri, stream: TlsStream) -> Self {
        Self {
            uri,
            stream: Mutex::new(Some(Stream::Tls(Box::new(stream)))),
        }
    }
    pub fn local(uri: Uri, stream: BridgeStream) -> Self {
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
        // TCP is already connected; this address is only library-required metadata, with no additional resolution or connection.
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
        // BridgeStream holds the absolute deadline for the entire request; do not refresh it per fragment.
        self.stream.write_all(&self.buffers.output()[..amount])?;
        self.stream.flush()?;
        Ok(())
    }
    fn await_input(&mut self, _: NextTimeout) -> Result<bool, ureq::Error> {
        let buffer = self.buffers.input_append_buf();
        // Requesting more input before consuming a full buffer is a capacity/protocol failure; a zero-length read must not falsely report remote EOF.
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
