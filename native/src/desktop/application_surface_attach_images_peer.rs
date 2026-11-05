//! Fixed loopback protocol peer; real attach and lifecycle ACKs precede resource delivery.
use super::*;
use base64::Engine;

pub(super) fn serve(
    listener: std::net::TcpListener,
    server: std::sync::Arc<rustls::ServerConfig>,
    revoke_ready: bool,
) -> std::net::TcpListener {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[12, 34, 56, 255, 90, 80, 70, 255])
            .unwrap();
    }
    let digest = format!("{:x}", Sha256::digest(&png));
    let resource_id = format!("sha256:{digest}");
    let inline = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&png)
    );
    let descriptor = serde_json::json!({"attachmentId":"attachment:test",
        "instanceId":"instance:test","hookNodeId":"node:test","deviceId":"device-old"});
    for stage in 0..if revoke_ready { 5 } else { 6 } {
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
                    serde_json::to_value(crate::desktop::surface_capabilities::report().unwrap())
                        .unwrap()
                );
                // 实际挂载请求须满足图片 Art 的声明，不能只与同一报告函数自比较。
                assert!(body["capabilities"]["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|node| node == "image"));
                assert_eq!(
                    body["capabilities"]["transports"],
                    serde_json::json!(["loom_resource"])
                );
                assert_eq!(
                    body["capabilities"]["capabilities"],
                    serde_json::json!(["remote_resources"])
                );
                assert_eq!(
                    body["capabilities"]["runtimes"],
                    serde_json::json!(["declarative"])
                );
                assert_eq!(body["artId"], "art:test");
                assert_eq!(body["hookNodeId"], "node:test");
                assert_eq!(body["deviceId"], "device-old");
                assert_eq!(body["persistence"], "temporary");
                let now = crate::desktop::surface_windows::now_ms().unwrap();
                serde_json::json!({"instance":{
                    "descriptor":{"instanceId":"instance:test","generation":1,"artId":"art:test"},
                    "attachments":{"attachment:test":{"descriptor":descriptor,
                        "lifecycle":"mounted","lifecycleRevision":1,
                        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"instance:test",
                            "attachmentId":"attachment:test","artId":"art:test","runtime":"declarative","revision":1,
                            "scene":{"id":"root","type":"column","children":[
                                {"id":"remote","type":"image","props":{"resourceId":resource_id}},
                                {"id":"missing","type":"image","props":{"resourceId":format!("sha256:{}", "0".repeat(64)),"src":inline}},
                                {"id":"inline","type":"image","props":{"src":inline}}
                            ]},
                            "resourceLeases":[{"leaseId":"lease:picture","resource":{"resourceId":resource_id,
                                "kind":"image","mime":"image/png","size":png.len(),"width":2,"height":1},
                                "transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{digest}")},
                                "expiresAtMs":now+60000}]
                        }}}
                }})
            }
            3 | 5 => {
                assert!(head.starts_with("POST /v1/surfaces/instances/instance:test/lifecycle "));
                let (state, revision) = if stage == 3 {
                    ("active", 2)
                } else {
                    ("inactive", 3)
                };
                assert_eq!(
                    body,
                    serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":"instance:test",
                    "attachmentId":"attachment:test","state":state,"revision":revision})
                );
                serde_json::json!({"descriptor":descriptor,"lifecycle":state,"lifecycleRevision":revision})
            }
            _ => {
                assert!(head.starts_with(&format!("GET /v1/surfaces/resources/{digest} ")));
                assert!(head
                    .to_ascii_lowercase()
                    .contains("x-loom-surface-lease: lease:picture"));
                reply(&mut stream, 200, &png);
                continue;
            }
        };
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    }
    listener
}
