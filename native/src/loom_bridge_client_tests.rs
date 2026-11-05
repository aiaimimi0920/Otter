//! 实际 TLS/WS 回环验证；不接触用户发现文件、凭据或运行中的 Loom。
use super::*;
use base64::Engine;
use rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig, ServerConnection};
use sha2::{Digest, Sha256};
use std::{io::Read, net::TcpListener, thread};
#[path = "loom_control_exchange_tests.rs"]
mod control_exchange;
#[cfg(windows)]
#[path = "loom_execution_job_tests.rs"]
pub(crate) mod execution_job;
#[cfg(windows)]
#[path = "loom_fragmented_session_tests.rs"]
mod fragmented_session;
#[path = "loom_handshake_exchange_tests.rs"]
mod handshake_exchange;
#[cfg(windows)]
#[path = "loom_session_tests.rs"]
mod session;
#[path = "loom_workflow_dispatch_tests.rs"]
mod workflow_dispatch;
#[path = "loom_workflow_exchange_tests.rs"]
mod workflow_exchange;
#[cfg(windows)]
#[path = "loom_workflow_sync_tests.rs"]
mod workflow_sync;

pub(crate) fn fixture() -> (TcpListener, Identity, Arc<ServerConfig>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let identity = Identity::validate(
        "loom.local-bridge.v1",
        "d2f4f738-09d9-4f46-829e-262381e63387",
        &format!("wss://127.0.0.1:{}/", listener.local_addr().unwrap().port()),
        &base64::engine::general_purpose::STANDARD.encode(cert.der()),
        &format!("{:x}", Sha256::digest(cert.der())),
        &"a1".repeat(32),
    )
    .unwrap();
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.der().clone()],
                PrivatePkcs8KeyDer::from(key_pair.serialize_der()).into(),
            )
            .unwrap();
    (listener, identity, Arc::new(config))
}

#[test]
// Tungstenite 的握手回调规定返回未装箱 HTTP Response，不能改变第三方接口。
#[allow(clippy::result_large_err)]
fn authenticated_tls13_websocket_roundtrip_uses_exporter_proof() {
    let (listener, identity, config) = fixture();
    let context = identity.exporter_context();
    let instance = identity.instance().to_owned();
    let worker = thread::spawn(move || {
        let mut stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        let mut tls = ServerConnection::new(config).unwrap();
        while tls.is_handshaking() {
            tls.complete_io(&mut stream).unwrap();
        }
        assert_eq!(
            tls.protocol_version(),
            Some(rustls::ProtocolVersion::TLSv1_3)
        );
        let proof = tls
            .export_keying_material(
                [0u8; 32],
                b"EXPORTER-Loom-Local-Bridge-v1",
                Some(context.as_bytes()),
            )
            .unwrap();
        let proof: String = proof.iter().map(|byte| format!("{byte:02x}")).collect();
        let mut socket = tungstenite::accept_hdr(
            StreamOwned::new(tls, stream),
            |request: &tungstenite::handshake::server::Request, response| {
                assert_eq!(
                    request.headers()["authorization"],
                    format!("LoomBridgeProof {proof}")
                );
                assert_eq!(request.headers()["x-loom-bridge-instance"], instance);
                assert!(!format!("{request:?}").contains(&"a1".repeat(32)));
                Ok(response)
            },
        )
        .unwrap();
        let message = socket.read().unwrap();
        socket.send(message).unwrap();
    });
    // 经过与默认连接相同的发现解析/身份校验，再进入真实 TLS/WS；不读取用户清单。
    let manifest = serde_json::to_vec(&serde_json::json!({
        "schemaVersion":1,"appId":"loom","displayName":"Loom","version":"1",
        "pid":1,"startedAt":1,"transport":{"type":"http","baseUrl":"http://127.0.0.1:80"},
        "capabilities":["brain.plan"],"hookBridge":{
            "protocol":"loom.local-bridge.v1","instanceId":identity.instance(),
            "endpoint":identity.endpoint(),
            "certificateDerBase64":base64::engine::general_purpose::STANDARD.encode(identity.certificate()),
            "certificateSha256":format!("{:x}", Sha256::digest(identity.certificate())),
            "authToken":"a1".repeat(32)
        }
    })).unwrap();
    let discovered = crate::loom_discovery::Config::from_lookup(|_| None)
        .load_with(|_| Ok(std::io::Cursor::new(&manifest)))
        .unwrap();
    let mut socket = connect(&discovered, Duration::from_secs(3)).unwrap();
    socket
        .send(tungstenite::Message::Text("authenticated".into()))
        .unwrap();
    assert_eq!(socket.read().unwrap().to_text().unwrap(), "authenticated");
    worker.join().unwrap();
}

#[test]
fn wrong_certificate_is_rejected_before_application_data() {
    let (listener, identity, _) = fixture();
    let (_, _, wrong_config) = fixture();
    let worker = thread::spawn(move || {
        let mut stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        let mut tls = ServerConnection::new(wrong_config).unwrap();
        while tls.is_handshaking() {
            if tls.complete_io(&mut stream).is_err() {
                break;
            }
        }
        assert!(!matches!(tls.reader().read(&mut [0; 1024]), Ok(count) if count > 0));
    });
    assert!(connect(&identity, Duration::from_secs(3)).is_err());
    worker.join().unwrap();
}

#[test]
fn invalid_timeout_rejected_before_connect() {
    let (_, identity, _) = fixture();
    assert!(matches!(
        connect(&identity, Duration::ZERO),
        Err(Error::InvalidTimeout)
    ));
    assert!(matches!(
        connect(&identity, Duration::MAX),
        Err(Error::InvalidTimeout)
    ));
}

#[test]
fn art_exchange_handles_ping_updates_and_terminal_result_on_real_tls_socket() {
    let (listener, identity, config) = fixture();
    let worker = thread::spawn(move || {
        let stream = BridgeStream::new(
            listener.accept().unwrap().0,
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
        let tls = ServerConnection::new(config).unwrap();
        let mut socket = tungstenite::accept(StreamOwned::new(tls, stream)).unwrap();
        assert_eq!(
            socket.read().unwrap().to_text().unwrap(),
            "execution-fixture"
        );
        socket
            .send(tungstenite::Message::Ping(vec![1, 2, 3]))
            .unwrap();
        assert_eq!(
            socket.read().unwrap(),
            tungstenite::Message::Pong(vec![1, 2, 3])
        );
        socket
            .send(tungstenite::Message::Text("unrelated".into()))
            .unwrap();
        let envelope = |method: &str, params: serde_json::Value| {
            serde_json::json!({
            "protocolVersion":"loom.hook.v1","requestId":"request","method":method,"params":params
        }).to_string()
        };
        socket
            .send(tungstenite::Message::Text(envelope(
                "loom.hook.art.progress",
                serde_json::json!({"value":0.5}),
            )))
            .unwrap();
        socket.send(tungstenite::Message::Text(envelope("loom.hook.art.preview", serde_json::json!({
            "protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,
            "previewRevision":1,"value":"preview"
        })))).unwrap();
        socket.send(tungstenite::Message::Text(envelope("loom.hook.art.result", serde_json::json!({
            "protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,
            "resultRevision":2,"outputs":{"output":"final"}
        })))).unwrap();
    });
    let socket = connect(&identity, Duration::from_secs(3)).unwrap();
    let request = crate::graph_requests::Request {
        id: crate::workspace_id::UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let mut updates = Vec::new();
    let result = crate::loom_art_exchange::run(
        socket,
        "execution-fixture".into(),
        &crate::workspace_id::UnitId::parse("unit").unwrap(),
        &request,
        Duration::from_secs(3),
        |update| {
            updates.push(update);
            true
        },
    )
    .unwrap();
    assert_eq!(
        updates,
        vec![
            crate::loom_art_response::Event::Progress(0.5),
            crate::loom_art_response::Event::Preview {
                revision: 1,
                value: serde_json::json!("preview")
            }
        ]
    );
    assert!(matches!(
        result,
        crate::loom_art_response::Event::Result { revision: 2, .. }
    ));
    worker.join().unwrap();
}

#[test]
fn art_exchange_stops_on_consumer_cancel_peer_close_or_total_timeout() {
    for mode in ["cancel", "close", "timeout"] {
        let (listener, identity, config) = fixture();
        let worker = thread::spawn(move || {
            let stream = BridgeStream::new(
                listener.accept().unwrap().0,
                Instant::now() + Duration::from_secs(3),
            )
            .unwrap();
            let mut socket = tungstenite::accept(StreamOwned::new(
                ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            socket.read().unwrap();
            if mode == "close" {
                socket.close(None).unwrap();
            } else if mode == "cancel" {
                socket.send(tungstenite::Message::Text(serde_json::json!({
                    "protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.progress","params":{"value":0.5}
                }).to_string())).unwrap();
                // 客户端回调拒绝后应释放 socket，无需等三秒读超时。
                assert!(socket.read().is_err());
            } else {
                // 连续无关消息不应续期总响应期限；客户端断开后服务线程退出。
                loop {
                    if socket
                        .send(tungstenite::Message::Text("ignored".into()))
                        .is_err()
                    {
                        break;
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });
        let socket = connect(&identity, Duration::from_secs(3)).unwrap();
        let request = crate::graph_requests::Request {
            id: crate::workspace_id::UnitId::parse("request").unwrap(),
            generation: 1,
        };
        let started = Instant::now();
        let result = crate::loom_art_exchange::run(
            socket,
            "fixture".into(),
            &crate::workspace_id::UnitId::parse("unit").unwrap(),
            &request,
            if mode == "timeout" {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(2)
            },
            |_| false,
        );
        let expected = match mode {
            "cancel" => crate::loom_art_exchange::Error::Cancelled,
            "close" => crate::loom_art_exchange::Error::Closed,
            _ => crate::loom_art_exchange::Error::Timeout,
        };
        assert_eq!(result, Err(expected));
        assert!(started.elapsed() < Duration::from_secs(3));
        worker.join().unwrap();
    }
}
