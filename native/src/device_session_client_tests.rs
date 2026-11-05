//! 真实 loopback TLS/HTTP 夹具，不访问用户凭据或在线 Loom。
use super::*;
use rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig, ServerConnection};
#[path = "device_approval_http_tests.rs"]
mod approval;
#[path = "device_authorization_owner_tests.rs"]
mod authorization_owner;
#[path = "device_authorization_runtime_http_tests.rs"]
mod authorization_runtime;
#[path = "live_relay_http_tests.rs"]
mod live_control;
#[path = "tile_http_tests.rs"]
mod tile;
#[path = "tile_surface_action_http_tests.rs"]
mod tile_actions;
#[path = "tile_terminal_failure_tests.rs"]
mod tile_failure;
#[path = "tile_identification_http_tests.rs"]
mod tile_identification;
#[path = "tile_surface_isolation_tests.rs"]
mod tile_isolation;
#[path = "tile_surface_http_tests.rs"]
mod tile_surface;
pub(crate) use live_control::session;
#[path = "device_pairing_http_tests.rs"]
pub(crate) mod pairing;
use std::{
    io::{self, Write},
    net::TcpListener,
};

fn fixture() -> (TcpListener, Client, Arc<ServerConfig>) {
    let (listener, client, config, _) = fixture_with_pem();
    (listener, client, config)
}
pub(crate) fn fixture_with_pem() -> (TcpListener, Client, Arc<ServerConfig>, String) {
    fixture_with_host("localhost")
}
/// 多次配置重解析的定向测试可选显式回环IP；默认仍覆盖localhost/SNI合同，不延长期限。
pub(crate) fn fixture_with_host(host: &str) -> (TcpListener, Client, Arc<ServerConfig>, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec![host.into()]).unwrap();
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
    let client = Client::new(
        &format!("https://{host}:{}", address.port()),
        roots,
        vec![address],
    )
    .unwrap();
    let pem = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, cert.der())
    );
    (listener, client, Arc::new(config), pem)
}
pub(crate) fn accept(listener: &TcpListener) -> BridgeStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((tcp, _)) => return BridgeStream::new(tcp, deadline).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("fixture: {e}"),
        }
    }
}
pub(crate) fn request(stream: &mut (impl Read + Write)) -> (String, serde_json::Value) {
    request_result(stream).unwrap()
}
// 取消型peer可接收真实socket关闭；旧入口仍unwrap，解析与大小断言不放宽。
pub(crate) fn request_result(
    stream: &mut (impl Read + Write),
) -> io::Result<(String, serde_json::Value)> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        assert!(head.len() < 16384);
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).unwrap();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let maximum = if head.starts_with("POST /v1/art/local/service/submit-images HTTP/1.1\r\n")
        || head.starts_with("POST /v1/art/local/service/submit-image-output HTTP/1.1\r\n")
    {
        crate::live_relay_http::art_prepare::service::MAX_IMAGE_REQUEST
    } else {
        16383
    };
    assert!(length <= maximum);
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    let body = if body.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    Ok((head, body))
}
pub(crate) fn reply(stream: &mut impl Write, status: u16, body: &[u8]) {
    reply_result(stream, status, body).unwrap();
}
pub(crate) fn reply_result(stream: &mut impl Write, status: u16, body: &[u8]) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

#[test]
fn actual_https_two_post_exchange_validates_proof_and_issues_session() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (listener, client, config) = fixture();
    let mut identity = Identity::generate().unwrap();
    identity.bind("device-1").unwrap();
    let public = STANDARD.decode(identity.public_key().unwrap()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/device-sessions/challenges HTTP/1.1"));
        assert_eq!(body, serde_json::json!({"deviceId":"device-1"}));
        reply(
            &mut stream,
            200,
            br#"{"deviceId":"device-1","challengeId":"id","challenge":"value"}"#,
        );
        drop(stream);
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/device-sessions HTTP/1.1"));
        let message = format!(
            "loom.device-session.v1\ndevice-1\nid\nvalue\n{}",
            body["clientNonce"].as_str().unwrap()
        );
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public)
            .verify(
                message.as_bytes(),
                &STANDARD
                    .decode(body["signature"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap();
        reply(&mut stream, 200, br#"{"deviceId":"device-1","token":"fixture-secret","expiresAtMs":18446744073709551615}"#);
    });
    let mut handles = Vec::new();
    let session = client
        .issue_once(&identity, &AtomicBool::new(false), |handle| {
            handles.push(handle)
        })
        .unwrap();
    assert_eq!(handles.len(), 2);
    assert_eq!(session.token_for(&client.origin, 0), Ok("fixture-secret"));
    peer.join().unwrap();
}

#[test]
fn actual_http_redirect_and_pending_are_returned_without_followup_requests() {
    for status in [302, 403] {
        let (listener, client, config) = fixture();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            let body = br#"{"error":{"code":"device_not_authorized"}}"#;
            write!(stream, "HTTP/1.1 {status} Fixture\r\nLocation: https://other.invalid/\r\nContent-Length: {}\r\n\r\n", body.len()).unwrap();
            stream.write_all(body).unwrap();
            stream.flush().unwrap();
        });
        let mut identity = Identity::generate().unwrap();
        identity.bind("device-1").unwrap();
        let result = client
            .issue_once(&identity, &AtomicBool::new(false), |_| {})
            .err();
        assert_eq!(
            result,
            Some(Error::Protocol(if status == 403 {
                device_session_attempt::Error::PendingApproval
            } else {
                device_session_attempt::Error::Http(status)
            }))
        );
        peer.join().unwrap();
    }
}

#[test]
fn actual_http_content_length_and_chunked_bodies_are_bounded() {
    for chunked in [false, true] {
        let (listener, client, config) = fixture();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            if chunked {
                let body = vec![b'x'; device_session_attempt::MAX_RESPONSE + 1];
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n",
                    body.len()
                )
                .unwrap();
                let _ = stream.write_all(&body);
                let _ = stream.write_all(b"\r\n0\r\n\r\n");
            } else {
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: 262145\r\n\r\n").unwrap();
            }
            let _ = stream.flush();
        });
        assert_eq!(
            client
                .post(
                    device_session_attempt::CHALLENGE_PATH,
                    b"{}",
                    &AtomicBool::new(false),
                    &mut |_| {}
                )
                .err(),
            Some(Error::Limit)
        );
        peer.join().unwrap();
    }
}

#[test]
fn actual_tls_untrusted_certificate_is_rejected() {
    let (listener, mut client, config) = fixture();
    client.tls = Arc::new(
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth(),
    );
    let peer = std::thread::spawn(move || {
        let mut tls = ServerConnection::new(config).unwrap();
        let mut stream = accept(&listener);
        assert!(tls.complete_io(&mut stream).is_err());
    });
    assert_eq!(
        client
            .post(
                device_session_attempt::CHALLENGE_PATH,
                b"{}",
                &AtomicBool::new(false),
                &mut |_| {}
            )
            .err(),
        Some(Error::Tls)
    );
    peer.join().unwrap();
}

#[test]
fn cancellation_before_tls_and_absolute_response_deadline_are_enforced() {
    let (listener, client, _) = fixture();
    let stop = AtomicBool::new(false);
    assert_eq!(
        client
            .post(
                device_session_attempt::CHALLENGE_PATH,
                b"{}",
                &stop,
                &mut |interrupt| {
                    stop.store(true, Ordering::Release);
                    interrupt.cancel().unwrap();
                }
            )
            .err(),
        Some(Error::Stopped)
    );
    drop(listener);
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        std::thread::sleep(Duration::from_millis(500));
    });
    assert_eq!(
        client
            .post_until(
                device_session_attempt::CHALLENGE_PATH,
                b"{}",
                &AtomicBool::new(false),
                &mut |_| {},
                Instant::now() + Duration::from_millis(200)
            )
            .err(),
        Some(Error::Deadline)
    );
    peer.join().unwrap();
}

#[test]
fn interrupt_aborts_an_actual_pending_http_response() {
    let (listener, client, config) = fixture();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        ready_tx.send(()).unwrap();
        let mut byte = [0];
        assert!(stream.read(&mut byte).map_or(true, |n| n == 0));
    });
    let stop = Arc::new(AtomicBool::new(false));
    let task_stop = stop.clone();
    let (handle_tx, handle_rx) = std::sync::mpsc::channel();
    let task = std::thread::spawn(move || {
        client
            .post(
                device_session_attempt::CHALLENGE_PATH,
                b"{}",
                &task_stop,
                &mut |handle| handle_tx.send(handle).unwrap(),
            )
            .err()
    });
    let handle = handle_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    stop.store(true, Ordering::Release);
    handle.cancel().unwrap();
    assert_eq!(task.join().unwrap(), Some(Error::Stopped));
    assert!(start.elapsed() < Duration::from_secs(2));
    peer.join().unwrap();
}

#[test]
fn actual_certificate_hostname_and_header_limits_are_enforced() {
    let (listener, mut client, config) = fixture();
    client.url = Url::parse(&format!(
        "https://127.0.0.1:{}",
        listener.local_addr().unwrap().port()
    ))
    .unwrap();
    let peer = std::thread::spawn(move || {
        let mut tls = ServerConnection::new(config).unwrap();
        assert!(tls.complete_io(&mut accept(&listener)).is_err());
    });
    assert_eq!(
        client
            .post(
                device_session_attempt::CHALLENGE_PATH,
                b"{}",
                &AtomicBool::new(false),
                &mut |_| {}
            )
            .err(),
        Some(Error::Tls)
    );
    peer.join().unwrap();
    let (listener, client, config) = fixture();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nX-Oversized: {}\r\nContent-Length: 0\r\n\r\n",
            "a".repeat(20 * 1024)
        );
        let _ = stream.flush();
    });
    assert_eq!(
        client
            .post(
                device_session_attempt::CHALLENGE_PATH,
                b"{}",
                &AtomicBool::new(false),
                &mut |_| {}
            )
            .err(),
        Some(Error::Http)
    );
    peer.join().unwrap();
}

#[test]
fn image_body_budget_never_expands_ordinary_or_unauthenticated_request_paths() {
    let (_listener, client, _config) = fixture();
    let body = vec![b' '; 1024 * 1024 + 1];
    for (method, path, device) in [
        (
            "POST",
            "/v1/art/local/service/submit",
            Some(("fixture", "nonce")),
        ),
        (
            "POST",
            "/v1/art/local/service/submit-images?admin=true",
            Some(("fixture", "nonce")),
        ),
        ("POST", "/v1/art/local/service/submit-images", None),
        (
            "GET",
            "/v1/art/local/service/submit-images",
            Some(("fixture", "nonce")),
        ),
        (
            "POST",
            "/v1/art/local/service/submit-image-output?admin=true",
            Some(("fixture", "nonce")),
        ),
        ("POST", "/v1/art/local/service/submit-image-output", None),
        (
            "GET",
            "/v1/art/local/service/submit-image-output",
            Some(("fixture", "nonce")),
        ),
        (
            "POST",
            "/v1/art/local/service/poll-image-output",
            Some(("fixture", "nonce")),
        ),
    ] {
        assert_eq!(
            client
                .exchange(
                    Request {
                        method,
                        path,
                        body: &body,
                        device,
                        limit: 1024
                    },
                    &AtomicBool::new(false),
                    &mut |_| {},
                    Instant::now() + Duration::from_millis(100)
                )
                .err(),
            Some(Error::Request)
        );
    }
}
