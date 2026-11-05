//! 临时自签证书与本机 TLS 服务；不读取用户凭据或联系实际 Loom。
use super::*;
use crate::{
    live_relay_media_endpoint::{Credential, Endpoint},
    loom_bridge_stream::BridgeStream,
};
use rustls::{
    pki_types::PrivatePkcs8KeyDer, RootCertStore, ServerConfig, ServerConnection, StreamOwned,
};
use std::{io::Read, net::TcpListener};

#[path = "live_relay_media_discovery_tests.rs"]
mod discovery;
#[path = "live_relay_source_socket_tests.rs"]
mod source;
#[path = "live_relay_source_video_socket_tests.rs"]
mod source_video;
#[path = "live_relay_media_trust_tests.rs"]
mod trust;

fn fixture() -> (TcpListener, RootCertStore, Arc<ServerConfig>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let mut roots = RootCertStore::empty();
    roots.add(cert.der().clone()).unwrap();
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.der().clone()],
                PrivatePkcs8KeyDer::from(key_pair.serialize_der()).into(),
            )
            .unwrap();
    (listener, roots, Arc::new(config))
}
fn accept(listener: TcpListener) -> BridgeStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((tcp, _)) => return BridgeStream::new(tcp, deadline).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("fixture accept: {e}"),
        }
    }
}

#[test]
#[allow(clippy::result_large_err)] // tungstenite 固定 callback 返回类型。
fn actual_tls_device_headers_negotiation_media_and_interrupt() {
    let (listener, roots, config) = fixture();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let tls = ServerConnection::new(config).unwrap();
        let mut socket = tungstenite::accept_hdr(
            StreamOwned::new(tls, accept(listener)),
            |request: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                assert_eq!(request.headers()["authorization"], "Device fixture-token");
                assert_eq!(
                    request.headers()["x-loom-device-nonce"].as_bytes().len(),
                    32
                );
                assert_eq!(request.uri().path(), "/v1/live/media");
                response
                    .headers_mut()
                    .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
                Ok(response)
            },
        )
        .unwrap();
        use crate::live_relay_frame::{Codec, ColorSpace, Metadata};
        let header = Metadata {
            epoch: 1,
            frame_id: 1,
            capture_timestamp_ms: 0,
            encode_timestamp_ms: 0,
            width: 1,
            height: 1,
            dropped_frames: 0,
            keyframe: true,
            color_space: ColorSpace::Srgb,
            codec: Codec::RawBgra,
        }
        .encode_header(4)
        .unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&[10, 20, 30, 255]);
        socket.send(tungstenite::Message::Binary(bytes)).unwrap();
        // 客户端 interrupt 应关闭底层 TCP，不等待服务端额外发送数据。
        assert!(socket.read().is_err());
    });
    let endpoint = Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
    let request = endpoint
        .viewer(
            "live:test",
            "viewer:test",
            (0, 0),
            Credential::Device("fixture-token"),
        )
        .unwrap();
    let mut connection = Connection::connect(request, &[address]).unwrap();
    assert_eq!(connection.profile(), Profile::Legacy);
    assert_eq!(connection.authorization, Authorization::Device);
    use crate::{
        live_relay_viewer_frames::Cursor, live_relay_viewer_identity::Snapshot,
        live_relay_viewer_session::ViewerSession,
    };
    let snapshot = Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false}"#, "live:test").unwrap();
    let session =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    session.activate(&snapshot).unwrap();
    let mut worker = Worker::start(session.clone(), session.admit(1).unwrap()).unwrap();
    let budget = Budget::new(1024);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if connection.service(&budget, &worker).unwrap() == Step::Queued {
            break;
        }
        assert!(Instant::now() < deadline);
    }
    loop {
        if let Some(frame) = session
            .take(Cursor {
                epoch: 1,
                frame_id: 1,
            })
            .unwrap()
        {
            assert_eq!(&**frame.pixels(), &[10, 20, 30, 255]);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    connection.interrupt_handle().unwrap().cancel().unwrap();
    assert_eq!(connection.service(&budget, &worker), Err(Error::Deadline));
    while worker.try_join().is_none() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    peer.join().unwrap();
}

#[test]
fn invalid_addresses_and_untrusted_certificate_fail_before_media() {
    let (listener, _, config) = fixture();
    let address = listener.local_addr().unwrap();
    let endpoint = Endpoint::new(
        &format!("https://localhost:{}", address.port()),
        RootCertStore::empty(),
    )
    .unwrap();
    let build = || {
        endpoint
            .viewer(
                "live:test",
                "viewer:test",
                (0, 0),
                Credential::Device("fixture-token"),
            )
            .unwrap()
    };
    assert!(matches!(
        Connection::connect(build(), &[]),
        Err(Error::Address)
    ));
    assert!(matches!(
        Connection::connect(build(), &[address; 5]),
        Err(Error::Address)
    ));
    let peer = std::thread::spawn(move || {
        let mut tls = StreamOwned::new(ServerConnection::new(config).unwrap(), accept(listener));
        assert!(tls.read(&mut [0; 1024]).is_err());
    });
    assert!(matches!(
        Connection::connect(build(), &[address]),
        Err(Error::Tls)
    ));
    peer.join().unwrap();
}

#[test]
#[allow(clippy::result_large_err)]
fn missing_or_unknown_selected_protocol_is_rejected() {
    for selected in [None, Some("unknown")] {
        let (listener, roots, config) = fixture();
        let address = listener.local_addr().unwrap();
        let peer = std::thread::spawn(move || {
            let tls = StreamOwned::new(ServerConnection::new(config).unwrap(), accept(listener));
            let _ = tungstenite::accept_hdr(
                tls,
                move |_: &tungstenite::handshake::server::Request,
                      mut response: tungstenite::handshake::server::Response| {
                    if let Some(selected) = selected {
                        response
                            .headers_mut()
                            .insert("sec-websocket-protocol", selected.parse().unwrap());
                    }
                    Ok(response)
                },
            );
        });
        let endpoint =
            Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
        let request = endpoint
            .viewer(
                "live:test",
                "viewer:test",
                (0, 0),
                Credential::Device("fixture-token"),
            )
            .unwrap();
        assert!(matches!(
            Connection::connect(request, &[address]),
            Err(Error::Upgrade | Error::Profile)
        ));
        peer.join().unwrap();
    }
}

#[test]
fn trusted_certificate_for_wrong_hostname_is_rejected() {
    let (listener, roots, config) = fixture();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut tls = StreamOwned::new(ServerConnection::new(config).unwrap(), accept(listener));
        assert!(tls.read(&mut [0; 1024]).is_err());
    });
    // 根可信但证书仅包含 localhost；解析地址相同不能绕过 hostname 检查。
    let endpoint =
        Endpoint::new(&format!("https://wrong.example:{}", address.port()), roots).unwrap();
    let request = endpoint
        .viewer(
            "live:test",
            "viewer:test",
            (0, 0),
            Credential::Device("fixture-token"),
        )
        .unwrap();
    assert!(matches!(
        Connection::connect(request, &[address]),
        Err(Error::Tls)
    ));
    peer.join().unwrap();
}

#[test]
fn stalled_tls_peer_obeys_total_handshake_deadline() {
    let (listener, roots, _) = fixture();
    let address = listener.local_addr().unwrap();
    let (release, wait) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let _stream = accept(listener);
        let _ = wait.recv_timeout(Duration::from_secs(5));
    });
    let endpoint = Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
    let request = endpoint
        .viewer(
            "live:test",
            "viewer:test",
            (0, 0),
            Credential::Device("fixture-token"),
        )
        .unwrap();
    let start = Instant::now();
    let result = Connection::connect(request, &[address]);
    let _ = release.send(());
    peer.join().unwrap();
    assert!(matches!(result, Err(Error::Tls)));
    assert!(start.elapsed() >= Duration::from_secs(2));
    assert!(start.elapsed() < Duration::from_secs(5));
}
