//! 真实TLS Source协商/严格策略/降级；固定wire独立oracle，不接实际设备。
use super::*;
#[test]
#[allow(clippy::result_large_err)]
fn source_h264_offer_receives_role_specific_policy_and_keeps_raw_available() {
    let (listener, roots, tls) = fixture();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut socket = tungstenite::accept_hdr(
            StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(listener)),
            |request: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                assert_eq!(
                    request.headers()["sec-websocket-protocol"],
                    "loom.live.h264.v1,loom.live.v1"
                );
                response.headers_mut().insert(
                    "sec-websocket-protocol",
                    "loom.live.h264.v1".parse().unwrap(),
                );
                Ok(response)
            },
        )
        .unwrap();
        socket
            .send(tungstenite::Message::Text(
                r#"{"type":"video_policy","epoch":1,"h264_allowed":true,"keyframe_sequence":3}"#
                    .into(),
            ))
            .unwrap();
        let bytes = socket.read().unwrap().into_data();
        assert_eq!(&bytes[..8], b"NLLV\x01\x01\x00\x40");
        assert_eq!(&bytes[56..64], &[1, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bytes[64..], &[5, 6, 7, 255].repeat(256 * 256));
        socket
            .send(tungstenite::Message::Text(
                r#"{"type":"keyframe_request","epoch":1}"#.into(),
            ))
            .unwrap();
        let _ = socket.read();
    });
    let endpoint = Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
    let mut prepared = endpoint
        .source("live:test", "source:test", Credential::Device("token"))
        .unwrap();
    prepared.source_h264();
    let mut connection = Connection::connect(prepared, &[address]).unwrap();
    connection.source_limits().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let SourceStep::Policy(policy) = connection.poll_source().unwrap() {
            assert_eq!(
                policy,
                VideoPolicy {
                    epoch: 1,
                    h264_allowed: true,
                    keyframe_sequence: 3
                }
            );
            break;
        }
        assert!(Instant::now() < deadline);
    }
    connection
        .send_source(
            super::source::metadata(),
            &[5, 6, 7, 255].repeat(256 * 256),
            || true,
        )
        .unwrap();
    loop {
        match connection.poll_source() {
            Ok(SourceStep::Idle) => {}
            Err(Error::Media(live_relay_media_socket::Error::Message)) => break,
            other => panic!("{other:?}"),
        }
        assert!(Instant::now() < deadline);
    }
    drop(connection);
    peer.join().unwrap();
}
#[test]
#[allow(clippy::result_large_err)]
fn source_cannot_accept_a_profile_it_did_not_offer() {
    let (listener, roots, tls) = fixture();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let _ = tungstenite::accept_hdr(
            StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(listener)),
            |_: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                response.headers_mut().insert(
                    "sec-websocket-protocol",
                    "loom.live.h264.v1".parse().unwrap(),
                );
                Ok(response)
            },
        );
    });
    let endpoint = Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
    let prepared = endpoint
        .source("live:test", "source:test", Credential::Device("token"))
        .unwrap();
    assert!(matches!(
        Connection::connect(prepared, &[address]),
        Err(Error::Profile | Error::Upgrade)
    ));
    peer.join().unwrap();
}
