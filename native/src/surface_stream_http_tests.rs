//! 本地真实 TLS 的设备 Surface 流读取；不请求管理员实例数据。
use super::*;
use crate::live_relay_http::StreamError;
#[test]
fn transport_rejects_response_budget_above_stream_ceiling_before_connect() {
    let (_listener, client, _config) = fixture();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let result = client.exchange(
        crate::device_session_client::Request {
            method: "GET",
            path: "/v1/surfaces/stream",
            body: &[],
            device: None,
            limit: crate::surface_stream::MAX_BYTES + 1,
        },
        &stop,
        &mut |_| panic!("invalid budget must not connect"),
        std::time::Instant::now() + std::time::Duration::from_secs(1),
    );
    assert!(matches!(
        result,
        Err(crate::device_session_client::Error::Request)
    ));
}
#[test]
fn surface_stream_uses_device_auth_and_preserves_cursor_and_reset() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for after in [0, 7] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(&format!(
                "GET /v1/surfaces/stream?after={after}&timeoutMs=0 "
            )));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret"));
            assert!(head.to_ascii_lowercase().contains("x-loom-device-nonce:"));
            assert!(body.is_null());
            let messages = if after == 0 {
                vec![
                    serde_json::json!({"method":"loom.surface.snapshot","params":{"payload":"x".repeat(600 * 1024)}});
                    2
                ]
            } else {
                vec![
                    serde_json::json!({"method":"loom.surface.action.ack","params":{"eventId":"event:test"}}),
                ]
            };
            reply(
                &mut stream,
                200,
                &serde_json::to_vec(&serde_json::json!({
                    "protocolVersion":"loom.surface-stream.v1","next":7,"reset":after == 0,
                    "messages":messages
                }))
                .unwrap(),
            );
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let batch = control.surface_stream(0).unwrap();
    assert_eq!(batch.next(), 7);
    assert!(batch.reset());
    assert_eq!(batch.messages().len(), 2);
    assert!(
        batch
            .messages()
            .iter()
            .map(|raw| raw.get().len())
            .sum::<usize>()
            > 1024 * 1024
    );
    let batch = control.surface_stream(7).unwrap();
    assert!(!batch.reset());
    assert_eq!(batch.next(), 7);
    peer.join().unwrap();
}
#[test]
fn surface_stream_unauthorized_response_does_not_fallback_or_retry() {
    let (listener, client, config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("GET /v1/surfaces/stream?after=9&timeoutMs=0 "));
        reply(&mut stream, 401, b"{}");
    });
    let cancel = Cancel::new();
    assert!(matches!(
        Control::new(&client, &session, &cancel).surface_stream(9),
        Err(StreamError::Transport(LiveError::AuthorizationRequired))
    ));
    peer.join().unwrap();
}
