//! 真实 TLS 固定会话 peer；逐项验证两个 emit 的 wire/nonce/顺序及每批后的两次观察。
use super::*;
pub(super) const SOURCE: &str = include_str!("../../tests/fixtures/javascript-batch.js");
pub(super) fn serve(
    listener: std::net::TcpListener,
    server: std::sync::Arc<rustls::ServerConfig>,
    mode: u8,
) -> std::net::TcpListener {
    let source = if mode == 20 {
        include_str!("../../tests/fixtures/javascript-hover-batch.js")
    } else if mode == 19 {
        include_str!("../../tests/fixtures/javascript-pointer-batch.js")
    } else if mode == 18 {
        include_str!("../../tests/fixtures/javascript-native-enter.js")
    } else if (15..=17).contains(&mode) {
        include_str!("../../tests/fixtures/javascript-native-focus.js")
    } else if mode == 14 {
        include_str!("../../tests/fixtures/javascript-native-input.js")
    } else if mode == 13 {
        include_str!("../../tests/fixtures/javascript-batch-events.js")
    } else if mode >= 11 {
        include_str!("../../tests/fixtures/javascript-batch-timers.js")
    } else if mode == 10 {
        include_str!("../../tests/fixtures/javascript-canvas.js")
    } else if mode >= 8 {
        include_str!("../../tests/fixtures/javascript-resize.js")
    } else if mode >= 6 {
        include_str!("../../tests/fixtures/javascript-batch-animation.js")
    } else if mode >= 3 {
        include_str!("../../tests/fixtures/javascript-batch-geometry.js")
    } else {
        SOURCE
    };
    let digest = format!("{:x}", Sha256::digest(source.as_bytes()));
    let descriptor = serde_json::json!({"attachmentId":"attachment:test","instanceId":"instance:test",
        "hookNodeId":"node:test","deviceId":"device-old"});
    let expiry = crate::desktop::surface_windows::now_ms().unwrap() + 60000;
    let snapshot = |revision| {
        serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":"instance:test",
            "attachmentId":"attachment:test","artId":"art:test","runtime":if mode == 2 && revision > 1 {"declarative"} else {"javascript"},
            "revision":revision,"state":{"quote":if revision == 1 {42} else if revision == 2 {45} else {48},"fail":mode == 1 && revision > 1},
            "entryResourceId":format!("sha256:{digest}"),"scene":{"id":"root","type":"column","children":[
                {"id":"authority","type":"text","props":{"text":"authority-not-rendered"}},
                {"id":"refresh","type":"button","events":{"click":"refresh"}}]},
            "resourceLeases":[{"leaseId":"lease:script","resource":{"resourceId":format!("sha256:{digest}"),
                "kind":"binary","mime":"application/javascript","size":source.len()},
                "transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{digest}")},"expiresAtMs":expiry}]
        })
    };
    let rounds = if matches!(mode, 0 | 3 | 6 | 11 | 13 | 14 | 15 | 18 | 19 | 20) {
        2
    } else if mode >= 4 {
        0
    } else {
        1
    };
    let mut nonces = std::collections::HashSet::new();
    let mut ids = std::collections::HashSet::new();
    let started = std::time::Instant::now();
    for stage in 0..(6 + rounds * 4) {
        let mut stream = StreamOwned::new(
            ServerConnection::new(server.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        eprintln!(
            "JS input peer stage {stage} at {:?}: {}",
            started.elapsed(),
            head.lines().next().unwrap()
        );
        if stage >= 2 {
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device configured-token"));
            let nonce = lower
                .lines()
                .find(|line| line.starts_with("x-loom-device-nonce:"))
                .unwrap();
            assert!(nonces.insert(nonce.to_owned()));
        }
        let response = match stage {
            0 => {
                assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                serde_json::json!({"deviceId":"device-old","challengeId":"id","challenge":"value"})
            }
            1 => {
                assert!(head.starts_with("POST /v1/device-sessions "));
                serde_json::json!({"deviceId":"device-old","token":"configured-token","expiresAtMs":u64::MAX})
            }
            2 => {
                assert!(head.starts_with("POST /v1/surfaces/attach "));
                assert_eq!(
                    body["capabilities"],
                    serde_json::to_value(super::peer::report()).unwrap()
                );
                serde_json::json!({"instance":{"descriptor":{"instanceId":"instance:test","generation":1,"artId":"art:test"},
                    "attachments":{"attachment:test":{"descriptor":descriptor,"lifecycle":"mounted","lifecycleRevision":1,"snapshot":snapshot(1)}}}})
            }
            3 => {
                assert!(head.starts_with(&format!("GET /v1/surfaces/resources/{digest} ")));
                assert!(head
                    .to_ascii_lowercase()
                    .contains("x-loom-surface-lease: lease:script"));
                reply(&mut stream, 200, source.as_bytes());
                continue;
            }
            4 => {
                assert!(head.starts_with("POST /v1/surfaces/instances/instance:test/lifecycle "));
                assert_eq!(body["state"], "active");
                assert_eq!(body["revision"], 2);
                serde_json::json!({"descriptor":descriptor,"lifecycle":"active","lifecycleRevision":2})
            }
            stage if stage == 5 + rounds * 4 => {
                assert!(head.starts_with("POST /v1/surfaces/instances/instance:test/lifecycle "));
                assert_eq!(body["state"], "inactive");
                assert_eq!(body["revision"], 3);
                serde_json::json!({"descriptor":descriptor,"lifecycle":"inactive","lifecycleRevision":3})
            }
            stage => {
                let round = (stage - 5) / 4;
                let phase = (stage - 5) % 4;
                if phase < 2 {
                    assert!(head.starts_with("POST /v1/surfaces/instances/instance:test/events "));
                    assert_eq!(body["nodeId"], "refresh");
                    assert_eq!(body["action"], "refresh");
                    assert_eq!(body["event"], "click");
                    assert_eq!(body["generation"], 1);
                    assert_eq!(body["baseRevision"], round + 1);
                    assert_eq!(
                        body["payload"],
                        serde_json::json!({"count":round + 1,"phase":phase + 1,"quote":if round == 0 {42} else {45}})
                    );
                    assert!(ids.insert(body["eventId"].as_str().unwrap().to_owned()));
                    serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
                        "eventId":body["eventId"],"requestId":format!("request:{round}:{phase}"),"accepted":true,"status":"succeeded"})
                } else {
                    assert!(head.starts_with("GET /v1/surfaces/stream?"));
                    assert!(head.contains("after=0"));
                    serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":round+1,"reset":true,
                        "messages":[{"method":"loom.surface.snapshot","params":{"generation":1,"snapshot":snapshot(round+2)}}]})
                }
            }
        };
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    }
    assert_eq!(ids.len(), rounds * 2);
    listener
}
