//! 原TLS/nonce/单次POST合同；不向终端开放管理员identify请求。
use super::*;
use crate::tile_identification::{Identity, Outcome, Report};
#[test]
fn exact_device_report_routes_preserve_identity_nonce_and_reject_false_ack() {
    let (listener, client, config) = fixture();
    let session = session(client.origin(), u64::MAX);
    let cancel = crate::device_request_cancel::Cancel::new();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for outcome in ["applied", "dismissed"] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/walls/endpoints/identify/report HTTP/1.1\r\n"));
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device fixture-secret\r\n"));
            assert!(nonces.insert(
                lower
                    .lines()
                    .find_map(|s| s.strip_prefix("x-loom-device-nonce: "))
                    .unwrap()
                    .to_owned()
            ));
            assert_eq!(
                body,
                serde_json::json!({"endpointId":"tile","leaseId":"lease","requestId":"11111111-1111-4111-8111-111111111111","outcome":outcome})
            );
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.wall.v1","accepted":outcome == "applied"})).unwrap());
        }
    });
    let control = crate::live_relay_http::Control::new(&client, &session, &cancel);
    for outcome in [Outcome::Applied, Outcome::Dismissed] {
        let report = Report {
            identity: Identity {
                endpoint: "tile".into(),
                lease: "lease".into(),
                request: "11111111-1111-4111-8111-111111111111".into(),
            },
            outcome,
        };
        assert_eq!(
            control.wall_identification_report(&report).is_ok(),
            outcome == Outcome::Applied
        );
    }
    peer.join().unwrap();
}
