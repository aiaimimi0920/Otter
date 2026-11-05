//! 分片发送真实TLS回环；接收独立核NLLV固定字节和像素，不用新发送器作oracle。
use super::*;
use crate::live_relay_frame::{Codec, ColorSpace, Metadata};
pub(super) fn metadata() -> Metadata {
    Metadata {
        epoch: 1,
        frame_id: 7,
        capture_timestamp_ms: 10,
        encode_timestamp_ms: 12,
        width: 256,
        height: 256,
        dropped_frames: 3,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
}
#[test]
#[allow(clippy::result_large_err)]
fn source_fragments_reassemble_exact_large_frame_without_expanding_write_buffer() {
    let (listener, roots, tls) = fixture();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut socket = tungstenite::accept_hdr(
            StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(listener)),
            |request: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                assert_eq!(request.headers()["sec-websocket-protocol"], "loom.live.v1");
                let url = url::Url::parse(&format!("https://test{}", request.uri())).unwrap();
                assert_eq!(
                    url.query_pairs().collect::<Vec<_>>(),
                    vec![
                        ("sessionId".into(), "live:test/part".into()),
                        ("role".into(), "source".into())
                    ]
                );
                assert_eq!(request.headers()["authorization"], "Device fixture-token");
                assert_eq!(
                    request.headers()["x-loom-device-nonce"].as_bytes().len(),
                    32
                );
                response
                    .headers_mut()
                    .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
                Ok(response)
            },
        )
        .unwrap();
        let message = socket.read().unwrap().into_data();
        assert_eq!(message.len(), 64 + 256 * 256 * 4);
        assert_eq!(&message[..8], b"NLLV\x01\x01\x00\x40");
        for (offset, expected) in [(8, 1u64), (16, 7), (24, 10), (32, 12)] {
            assert_eq!(&message[offset..offset + 8], &expected.to_be_bytes());
        }
        for (offset, expected) in [(40, 256u32), (44, 256), (48, 3), (52, 256 * 256 * 4)] {
            assert_eq!(&message[offset..offset + 4], &expected.to_be_bytes());
        }
        assert_eq!(&message[56..64], &[1, 1, 0, 0, 0, 0, 0, 0]);
        for (i, byte) in message[64..].iter().enumerate() {
            assert_eq!(*byte, (i % 251) as u8);
        }
    });
    let endpoint = Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
    let prepared = endpoint
        .source(
            "live:test/part",
            "source:test",
            Credential::Device("fixture-token"),
        )
        .unwrap();
    let mut connection = Connection::connect(prepared, &[address]).unwrap();
    connection.source_limits().unwrap();
    assert_eq!(
        connection.socket.get_config().max_write_buffer_size,
        64 * 1024
    );
    assert_eq!(
        connection.socket.get_config().max_message_size,
        Some(64 * 1024)
    );
    connection
        .send_source(
            metadata(),
            &(0..256 * 256 * 4)
                .map(|i| (i % 251) as u8)
                .collect::<Vec<_>>(),
            || true,
        )
        .unwrap();
    peer.join().unwrap();
}
#[test]
#[allow(clippy::result_large_err)]
fn cancellation_between_fragments_never_publishes_a_complete_frame() {
    let (listener, roots, tls) = fixture();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut socket = tungstenite::accept_hdr(
            StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(listener)),
            |_: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                response
                    .headers_mut()
                    .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
                Ok(response)
            },
        )
        .unwrap();
        assert!(socket.read().is_err());
    });
    let endpoint = Endpoint::new(&format!("https://localhost:{}", address.port()), roots).unwrap();
    let mut connection = Connection::connect(
        endpoint
            .source(
                "live:test",
                "source:test",
                Credential::Device("fixture-token"),
            )
            .unwrap(),
        &[address],
    )
    .unwrap();
    connection.source_limits().unwrap();
    let calls = std::cell::Cell::new(0);
    assert_eq!(
        connection.send_source(metadata(), &vec![7; 256 * 256 * 4], || {
            let n = calls.get();
            calls.set(n + 1);
            n == 0
        }),
        Err(Error::Stopped)
    );
    drop(connection);
    peer.join().unwrap();
}
#[test]
fn source_protocol_and_capacity_failures_do_not_become_retryable_transport() {
    use std::io::ErrorKind;
    for (kind, expected) in [
        (
            ErrorKind::ConnectionReset,
            live_relay_media_socket::Error::Transport,
        ),
        (
            ErrorKind::TimedOut,
            live_relay_media_socket::Error::Transport,
        ),
        (
            ErrorKind::InvalidData,
            live_relay_media_socket::Error::Message,
        ),
        (
            ErrorKind::PermissionDenied,
            live_relay_media_socket::Error::Message,
        ),
    ] {
        assert_eq!(
            live_relay_media_socket::Error::from_socket(tungstenite::Error::Io(
                std::io::Error::from(kind)
            )),
            expected
        );
    }
}
