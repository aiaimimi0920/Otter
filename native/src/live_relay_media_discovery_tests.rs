//! 真实 loopback HTTP upgrade 验证清单→请求→连接，不模拟远端 Device 签发。
use super::*;

#[test]
#[allow(clippy::result_large_err)]
fn discovered_local_bearer_reaches_actual_websocket_without_bridge_identity() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut socket = tungstenite::accept_hdr(
            accept(listener),
            |request: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                assert_eq!(request.headers()["authorization"], "Bearer local-fixture");
                assert!(request.headers().get("x-loom-device-nonce").is_none());
                let url = url::Url::parse(&format!("http://localhost{}", request.uri())).unwrap();
                let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
                assert_eq!(query["deviceId"], "device-000-local");
                assert_eq!(query["afterEpoch"], "3");
                assert_eq!(query["afterFrameId"], "7");
                response.headers_mut().insert(
                    "sec-websocket-protocol",
                    "loom.live.h264.v1".parse().unwrap(),
                );
                Ok(response)
            },
        )
        .unwrap();
        socket.close(None).unwrap();
    });
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schemaVersion":1,"appId":"loom","displayName":"Loom","version":"1","pid":1,"startedAt":1,
        "capabilities":["brain.plan"], "transport":{"type":"http",
            "baseUrl":format!("http://localhost:{}", address.port()),"auth":"bearer","authToken":" local-fixture "}
    })).unwrap();
    let config = crate::loom_discovery::Config::from_lookup(|_| None);
    let target = config
        .load_media_with(|_| Ok(std::io::Cursor::new(&bytes)))
        .unwrap();
    let request = target
        .local_viewer("live:test", (3, 7), RootCertStore::empty())
        .unwrap();
    let mut connection = Connection::connect(request, &[address]).unwrap();
    assert_eq!(connection.profile(), Profile::H264);
    assert_eq!(connection.authorization, Authorization::Other);
    assert!(matches!(
        connection.socket.read(),
        Ok(tungstenite::Message::Close(_))
    ));
    peer.join().unwrap();
}

#[test]
fn localhost_never_connects_to_remote_resolution_for_plain_or_tls() {
    for scheme in ["http", "https"] {
        let endpoint = Endpoint::new(
            &format!("{scheme}://localhost:1234"),
            RootCertStore::empty(),
        )
        .unwrap();
        let prepared = endpoint
            .viewer(
                "live:test",
                "device-000-local",
                (0, 0),
                Credential::Bearer("local-secret"),
            )
            .unwrap();
        assert!(matches!(
            Connection::connect(prepared, &["203.0.113.1:1234".parse().unwrap()]),
            Err(Error::Address)
        ));
    }
}
