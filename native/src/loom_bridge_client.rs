//! 固定证书的本机 TLS 1.3/WebSocket 连接；仅后台调用，发现来源由调用方负责。
use crate::{loom_bridge_identity::Identity, loom_bridge_stream::BridgeStream};
use rustls::{
    pki_types::{CertificateDer, ServerName},
    ClientConfig, ClientConnection, RootCertStore, StreamOwned,
};
use std::{
    net::TcpStream,
    sync::Arc,
    time::{Duration, Instant},
};
use tungstenite::{client::IntoClientRequest, http::HeaderValue, WebSocket};

pub type BridgeSocket = WebSocket<StreamOwned<ClientConnection, BridgeStream>>;
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(3);
/// 不传播底层 HTTP/TLS 错误文本，避免意外暴露认证头或发现凭据。
#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidTimeout,
    Connect,
    Certificate,
    Tls,
    Proof,
    Upgrade,
}

pub fn connect(identity: &Identity, operation_timeout: Duration) -> Result<BridgeSocket, Error> {
    if operation_timeout.is_zero() || Instant::now().checked_add(operation_timeout).is_none() {
        return Err(Error::InvalidTimeout);
    }
    let mut trust = RootCertStore::empty();
    trust
        .add(CertificateDer::from(identity.certificate().to_vec()))
        .map_err(|_| Error::Certificate)?;
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| Error::Tls)?
            .with_root_certificates(trust)
            .with_no_client_auth();
    let deadline = Instant::now() + HANDSHAKE_LIMIT;
    let tcp = TcpStream::connect_timeout(&identity.address(), HANDSHAKE_LIMIT)
        .map_err(|_| Error::Connect)?;
    let mut stream = BridgeStream::new(tcp, deadline).map_err(|_| Error::Connect)?;
    let mut tls = ClientConnection::new(
        Arc::new(config),
        ServerName::try_from("localhost").map_err(|_| Error::Tls)?,
    )
    .map_err(|_| Error::Tls)?;
    while tls.is_handshaking() {
        tls.complete_io(&mut stream).map_err(|_| Error::Tls)?;
    }
    if tls
        .peer_certificates()
        .and_then(|chain| chain.first())
        .is_none_or(|leaf| leaf.as_ref() != identity.certificate())
    {
        return Err(Error::Certificate);
    }
    let context = identity.exporter_context();
    let proof = tls
        .export_keying_material(
            [0u8; 32],
            b"EXPORTER-Loom-Local-Bridge-v1",
            Some(context.as_bytes()),
        )
        .map_err(|_| Error::Proof)?;
    let proof: String = proof.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut request = identity
        .endpoint()
        .into_client_request()
        .map_err(|_| Error::Upgrade)?;
    let mut auth =
        HeaderValue::from_str(&format!("LoomBridgeProof {proof}")).map_err(|_| Error::Proof)?;
    auth.set_sensitive(true);
    request.headers_mut().insert("authorization", auth);
    request.headers_mut().insert(
        "x-loom-bridge-instance",
        identity.instance().parse().map_err(|_| Error::Upgrade)?,
    );
    let config = tungstenite::protocol::WebSocketConfig {
        max_message_size: Some(64 * 1024 * 1024),
        max_frame_size: Some(64 * 1024 * 1024),
        max_write_buffer_size: 65 * 1024 * 1024,
        ..Default::default()
    };
    let (mut socket, _) = tungstenite::client::client_with_config(
        request,
        StreamOwned::new(tls, stream),
        Some(config),
    )
    .map_err(|_| Error::Upgrade)?;
    socket
        .get_mut()
        .sock
        .operation_deadline(operation_timeout)
        .map_err(|_| Error::InvalidTimeout)?;
    Ok(socket)
}

#[cfg(test)]
#[path = "loom_bridge_client_tests.rs"]
pub(crate) mod tests;
