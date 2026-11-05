//! 原回环 TLS fixture 的严格四步对端；只提供本轮实际操作，不假造客户端完成或撤权。
use super::*;
pub(super) fn serve(
    listener: std::net::TcpListener,
    tls: Arc<rustls::ServerConfig>,
    origin: String,
    mode: u8,
    ready: mpsc::SyncSender<()>,
) -> std::net::TcpListener {
    serve_routed(listener, tls, origin, mode, ready, false)
}
pub(super) fn serve_routed(
    listener: std::net::TcpListener,
    tls: Arc<rustls::ServerConfig>,
    origin: String,
    mode: u8,
    ready: mpsc::SyncSender<()>,
    offline: bool,
) -> std::net::TcpListener {
    let started = Instant::now();
    for index in 0..if mode <= 1 { 4 } else { 3 } {
        eprintln!(
            "projection-inbox-peer: phase={index} wait elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let mut stream = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        eprintln!(
            "projection-inbox-peer: phase={index} request elapsed_ms={}",
            started.elapsed().as_millis()
        );
        match index {
            0 => {
                assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                reply(
                    &mut stream,
                    200,
                    br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                );
            }
            1 => {
                assert!(head.starts_with("POST /v1/device-sessions "));
                reply(&mut stream, 200, br#"{"deviceId":"device-old","token":"projection-token","expiresAtMs":18446744073709551615}"#);
            }
            2 => {
                assert!(head.starts_with("POST /v1/projections/inbox "));
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device projection-token\r\n"));
                assert_eq!(body, serde_json::json!({"policy":"confirm"}));
                if offline {
                    reply(
                        &mut stream,
                        200,
                        br#"{"invitations":[],"offlinePeers":true}"#,
                    );
                    drop(stream);
                    stream = StreamOwned::new(
                        ServerConnection::new(tls.clone()).unwrap(),
                        accept(&listener),
                    );
                    let (head, body) = request(&mut stream);
                    assert!(head.starts_with("POST /v1/offline-projections/inbox "));
                    assert!(head
                        .to_ascii_lowercase()
                        .contains("authorization: device projection-token\r\n"));
                    assert_eq!(body, serde_json::json!({"policy":"confirm"}));
                }
                if mode == 5 {
                    ready.send(()).unwrap();
                    let mut byte = [0];
                    assert!(!stream.read(&mut byte).is_ok_and(|n| n != 0));
                } else {
                    reply(
                        &mut stream,
                        200,
                        &if offline {
                            crate::projection_delivery::tests::offline::body(false)
                        } else {
                            crate::projection_delivery::tests::body(&origin)
                        },
                    );
                }
            }
            3 => {
                assert!(head.starts_with(if offline {
                    "POST /v1/offline-projections/receipt "
                } else {
                    "POST /v1/projections/receipt "
                }));
                assert!(head
                    .to_ascii_lowercase()
                    .contains("authorization: device projection-token\r\n"));
                assert_eq!(
                    body,
                    serde_json::json!({"projectionId":format!("projection:{}", "1".repeat(32)),"status":"rejected"})
                );
                reply(
                    &mut stream,
                    200,
                    if mode == 0 {
                        br#"{"recorded":true}"#
                    } else {
                        br#"{"recorded":false}"#
                    },
                );
            }
            _ => unreachable!(),
        }
    }
    listener
}
