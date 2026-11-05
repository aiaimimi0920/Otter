//! 本批真实LAN联合的测试客户端；不增加产品source入口、不读取真实身份。
use super::*;
use std::net::TcpStream;
use tungstenite::{client::IntoClientRequest, http::HeaderValue, Message};
pub(super) type Socket = tungstenite::WebSocket<
    rustls::StreamOwned<rustls::ClientConnection, crate::loom_bridge_stream::BridgeStream>,
>;

pub(super) fn connect(
    origin: &str,
    roots: RootCertStore,
    token: &str,
    role: &str,
    nonce: &str,
) -> Result<Socket, Box<tungstenite::Error>> {
    let url = url::Url::parse(origin).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let tcp = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", url.port().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(2),
    )
    .map_err(|error| Box::new(tungstenite::Error::Io(error)))?;
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let tls = rustls::ClientConnection::new(
        Arc::new(config),
        rustls::pki_types::ServerName::try_from("127.0.0.1".to_owned()).unwrap(),
    )
    .unwrap();
    let base =
        crate::loom_bridge_stream::BridgeStream::new(tcp, Instant::now() + Duration::from_secs(3))
            .map_err(|error| Box::new(tungstenite::Error::Io(error)))?;
    let stream = rustls::StreamOwned::new(tls, base);
    let mut request = format!(
        "wss://127.0.0.1:{}/v1/live/media?sessionId=live%3Alan%2Fpart&role={role}",
        url.port().unwrap()
    )
    .into_client_request()
    .map_err(Box::new)?;
    for (name, value) in [
        ("authorization", format!("Device {token}")),
        (
            "x-loom-device-nonce",
            format!("{nonce}-isolated-test-nonce"),
        ),
        ("sec-websocket-protocol", "loom.live.v1".into()),
    ] {
        request
            .headers_mut()
            .insert(name, HeaderValue::from_str(&value).unwrap());
    }
    let (mut socket, response) =
        tungstenite::client(request, stream).map_err(|error| match error {
            tungstenite::HandshakeError::Failure(error) => Box::new(error),
            tungstenite::HandshakeError::Interrupted(_) => panic!("blocking handshake interrupted"),
        })?;
    assert_eq!(response.headers()["sec-websocket-protocol"], "loom.live.v1");
    socket
        .get_mut()
        .sock
        .operation_deadline(Duration::from_millis(250))
        .map_err(|error| Box::new(tungstenite::Error::Io(error)))?;
    Ok(socket)
}

pub(super) fn frame() -> Vec<u8> {
    use crate::live_relay_frame::*;
    let mut bytes = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width: 2,
        height: 2,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
    .encode_header(16)
    .unwrap()
    .to_vec();
    bytes.extend([17, 83, 211, 255].repeat(4));
    bytes
}
pub(super) fn binary(socket: &mut Socket) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < deadline, "real LAN binary frame deadline");
        socket
            .get_mut()
            .sock
            .operation_deadline(Duration::from_millis(250))
            .unwrap();
        match socket.read() {
            Ok(Message::Binary(bytes)) => {
                assert_eq!(bytes, frame());
                return;
            }
            Ok(Message::Ping(bytes)) => socket.send(Message::Pong(bytes)).unwrap(),
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            other => panic!("expected real LAN binary: {other:?}"),
        }
    }
}
pub(super) fn closed(socket: &mut Socket, revoked: bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < deadline, "LAN socket cleanup deadline");
        socket
            .get_mut()
            .sock
            .operation_deadline(Duration::from_millis(250))
            .unwrap();
        match socket.read() {
            Ok(Message::Close(close)) => {
                if revoked {
                    let close = close.unwrap();
                    assert_eq!(
                        close.code,
                        tungstenite::protocol::frame::coding::CloseCode::Policy
                    );
                    assert_eq!(close.reason, "live_media_device_revoked");
                }
                return;
            }
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(_) if !revoked => return,
            Ok(Message::Ping(bytes)) => {
                let _ = socket.send(Message::Pong(bytes));
            }
            other => panic!("LAN socket must close without more frames: {other:?}"),
        }
    }
}

pub(super) struct Apartment;
impl Apartment {
    pub fn new() -> Self {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}
pub(super) fn pump() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
