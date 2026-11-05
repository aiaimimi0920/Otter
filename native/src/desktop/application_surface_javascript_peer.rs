//! 固定 loopback TLS 协议 peer；核实 GET 在 active 之前，不替代真实 Loom/设备批准。
use super::*;
pub(crate) fn report() -> crate::surface_host_capabilities::Report {
    use crate::surface_host_capabilities::{Input, Report, Runtime};
    Report::new(
        vec![Runtime::Javascript],
        vec!["column".into(), "text".into(), "button".into()],
        vec!["loom_resource".into()],
        vec!["remote_resources".into()],
        Input {
            pointer: true,
            keyboard: true,
            ..Default::default()
        },
    )
    .unwrap()
}
pub(super) fn serve(
    listener: std::net::TcpListener,
    server: std::sync::Arc<rustls::ServerConfig>,
    advertised: bool,
    revoke: bool,
    corrupt: bool,
    download: std::sync::mpsc::Receiver<()>,
) -> std::net::TcpListener {
    let source = crate::surface_javascript::fixture::SOURCE.as_bytes();
    let digest = format!("{:x}", Sha256::digest(source));
    let descriptor = serde_json::json!({"attachmentId":"attachment:test", "instanceId":"instance:test",
        "hookNodeId":"node:test", "deviceId":"device-old"});
    let count = if !advertised {
        3
    } else if revoke || corrupt {
        5
    } else {
        6
    };
    for stage in 0..count {
        let mut stream = StreamOwned::new(
            ServerConnection::new(server.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut stream);
        if stage >= 2 {
            let lower = head.to_ascii_lowercase();
            assert!(lower.contains("authorization: device configured-token"));
            assert!(lower.contains("x-loom-device-nonce:"));
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
                    serde_json::to_value(if advertised {
                        report()
                    } else {
                        crate::desktop::surface_capabilities::report().unwrap()
                    })
                    .unwrap()
                );
                let now = crate::desktop::surface_windows::now_ms().unwrap();
                serde_json::json!({"instance":{"descriptor":{"instanceId":"instance:test","generation":1,"artId":"art:test"},
                "attachments":{"attachment:test":{"descriptor":descriptor,"lifecycle":"mounted","lifecycleRevision":1,
                "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"instance:test","attachmentId":"attachment:test",
                        "artId":"art:test","runtime":"javascript","revision":1,"state":{"quote":42},"entryResourceId":format!("sha256:{digest}"),
                    "scene":{"id":"root","type":"column","children":[
                        {"id":"authority","type":"text","props":{"text":"authority-not-rendered"}},
                        {"id":"refresh","type":"button","events":{"click":"refresh"}}]},
                    "resourceLeases":[{"leaseId":"lease:script","resource":{"resourceId":format!("sha256:{digest}"),
                        "kind":"binary","mime":"application/javascript","size":source.len()},
                        "transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{digest}")},"expiresAtMs":now+60000}]
                }}}}})
            }
            3 => {
                assert!(head.starts_with(&format!("GET /v1/surfaces/resources/{digest} ")));
                assert!(head
                    .to_ascii_lowercase()
                    .contains("x-loom-surface-lease: lease:script"));
                download
                    .recv_timeout(std::time::Duration::from_secs(8))
                    .unwrap();
                reply(&mut stream, 200, if corrupt { b"corrupt" } else { source });
                continue;
            }
            _ => {
                assert!(head.starts_with("POST /v1/surfaces/instances/instance:test/lifecycle "));
                let (state, revision) = if stage == 4 && !corrupt {
                    ("active", 2)
                } else {
                    ("inactive", if corrupt { 2 } else { 3 })
                };
                assert_eq!(
                    body,
                    serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":"instance:test",
                    "attachmentId":"attachment:test","state":state,"revision":revision})
                );
                serde_json::json!({"descriptor":descriptor,"lifecycle":state,"lifecycleRevision":revision})
            }
        };
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    }
    listener
}
