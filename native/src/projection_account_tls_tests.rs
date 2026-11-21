//! 当前授权 Loom 的 TLS 边界；覆盖地址别名、真实大图、401 及 socket 取消。
use super::*;
use crate::device_session_client::tests::fixture_with_pem;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use std::io::{Read, Write};

fn stream(
    listener: &TcpListener,
    config: Arc<ServerConfig>,
) -> StreamOwned<ServerConnection, crate::loom_bridge_stream::BridgeStream> {
    StreamOwned::new(ServerConnection::new(config).unwrap(), accept(listener))
}

#[test]
fn authorized_tls_accepts_advertised_alias_and_large_png_on_fixed_loom() {
    let image = images::large_image();
    let envelope = envelope(&image);
    let result = wire(&envelope, Some(&image), 1, image.digest(), None);
    let (listener, client, config, _) = fixture_with_pem();
    let credential = session(client.origin(), now().unwrap() + 120_000);
    let peer = std::thread::spawn(move || {
        let mut wire = stream(&listener, config.clone());
        let (head, _) = request(&mut wire);
        assert!(head.starts_with("GET /v1/otter/identity "));
        assert!(!head.to_ascii_lowercase().contains("authorization:"));
        reply(
            &mut wire,
            200,
            &serde_json::to_vec(&json!({"protocolVersion":"loom.connection.v1",
            "endpoint":"https://192.168.15.20:8766","loomId":format!("loom-{}", "a".repeat(64))}))
            .unwrap(),
        );
        drop(wire);
        for (path, value) in [("context", context_json(SOURCE)), ("create", result)] {
            let mut wire = stream(&listener, config.clone());
            let (head, body) = request(&mut wire);
            assert!(head.starts_with(&format!("POST /v1/projections/v2/{path} ")));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret"));
            if path == "create" {
                assert!(body["snapshot"]["imageBase64"].as_str().unwrap().len() > 1024 * 1024);
            }
            reply(&mut wire, 200, &serde_json::to_vec(&value).unwrap());
        }
    });
    let cancel = Cancel::new();
    let account = Account::authorized(client, &credential, &cancel).unwrap();
    let State::Content(content) = account
        .execute(Operation::Create {
            unit: "unit-1",
            kind: "sticker",
            image: &image,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(content.image.unwrap().digest(), image.digest());
    assert!(!cancel.test_disconnect());
    peer.join().unwrap();
}

#[test]
fn authorized_tls_keeps_signed_out_and_malformed_401_status() {
    for mode in 0..3 {
        let (listener, client, config, _) = fixture_with_pem();
        let credential = session(client.origin(), now().unwrap() + 120_000);
        let peer = std::thread::spawn(move || {
            let mut wire = stream(&listener, config);
            request(&mut wire);
            match mode {
                0 => reply(
                    &mut wire,
                    401,
                    br#"{"error":{"code":"account_signed_out"}}"#,
                ),
                1 => {
                    wire.write_all(b"HTTP/1.1 401 Fixture\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{").unwrap();
                    wire.flush().unwrap();
                }
                _ => {
                    wire.write_all(b"HTTP/1.1 401 Fixture\r\nContent-Length: 999999\r\nConnection: close\r\n\r\n").unwrap();
                    wire.flush().unwrap();
                }
            }
        });
        let endpoint = http::Endpoint::authorized(client);
        let cancel = Cancel::new();
        let error = endpoint
            .request(
                "POST",
                "/v1/projections/v2/context",
                b"{}",
                Some(&credential),
                &cancel,
                1024,
            )
            .err()
            .unwrap();
        assert_eq!(
            error,
            Error::Remote {
                status: 401,
                code: (mode == 0).then(|| "account_signed_out".into())
            }
        );
        assert!(!cancel.test_disconnect());
        peer.join().unwrap();
    }
}

#[test]
fn authorized_tls_cancel_interrupts_pending_response() {
    let (listener, client, config, _) = fixture_with_pem();
    let credential = session(client.origin(), now().unwrap() + 120_000);
    let endpoint = http::Endpoint::authorized(client);
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(Cancel::new());
    let stop = cancel.clone();
    let peer = std::thread::spawn(move || {
        let mut wire = stream(&listener, config);
        request(&mut wire);
        tx.send(()).unwrap();
        assert!(!wire.read(&mut [0u8; 1]).is_ok_and(|n| n > 0));
    });
    let worker = std::thread::spawn(move || {
        endpoint
            .request(
                "POST",
                "/v1/projections/v2/context",
                b"{}",
                Some(&credential),
                &stop,
                1024,
            )
            .err()
            .unwrap()
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    cancel.cancel();
    assert_eq!(worker.join().unwrap(), Error::Stopped);
    assert!(!cancel.test_disconnect());
    peer.join().unwrap();
}
