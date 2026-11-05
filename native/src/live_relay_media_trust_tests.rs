//! 操作员 PEM 加载后的真实 TLS 验证；跨端口不能复用附加 CA。
use super::*;
use crate::loom_tls_trust::Trust;
use base64::Engine;

#[test]
#[allow(clippy::result_large_err)]
fn operator_ca_load_reaches_tls_only_for_its_exact_origin() {
    for matching in [true, false] {
        let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("https://localhost:{}", address.port());
        let pem = format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            base64::engine::general_purpose::STANDARD.encode(identity.certificate())
        );
        let configured = if matching {
            origin.clone()
        } else {
            "https://localhost:1".to_owned()
        };
        let path = std::env::current_dir()
            .unwrap()
            .join("operator-ca-fixture.pem");
        let trust = Trust::load_with(
            RootCertStore::empty(),
            |name| match name {
                "HOOK_LOOM_TLS_ORIGIN" => Some(configured.clone().into()),
                "HOOK_LOOM_TLS_CA_FILE" => Some(path.clone().into_os_string()),
                _ => None,
            },
            |_| Ok(std::io::Cursor::new(pem.as_bytes())),
        )
        .unwrap();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schemaVersion":1,"appId":"loom","displayName":"Loom","version":"1","pid":1,"startedAt":1,
            "capabilities":["brain.plan"],"transport":{"type":"http","baseUrl":origin,"auth":"none"}
        })).unwrap();
        let target = crate::loom_discovery_manifest::parse_media(&bytes).unwrap();
        let request = target
            .local_viewer_with_trust("live:test", (0, 0), &trust)
            .unwrap();
        let peer = std::thread::spawn(move || {
            let tls = StreamOwned::new(ServerConnection::new(config).unwrap(), accept(listener));
            let result = tungstenite::accept_hdr(
                tls,
                |request: &tungstenite::handshake::server::Request,
                 mut response: tungstenite::handshake::server::Response| {
                    assert!(matching);
                    assert!(request.headers().get("authorization").is_none());
                    response
                        .headers_mut()
                        .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
                    Ok(response)
                },
            );
            assert_eq!(result.is_ok(), matching);
        });
        let result = Connection::connect(request, &[address]);
        if matching {
            assert_eq!(result.unwrap().profile(), Profile::Legacy);
        } else {
            assert!(matches!(result, Err(Error::Tls)));
        }
        peer.join().unwrap();
    }
}
