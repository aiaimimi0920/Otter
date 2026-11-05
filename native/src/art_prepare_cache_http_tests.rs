//! 缓存命中仍经真实TLS取新许可；错误不回退为执行旧许可。
use super::*;
#[derive(Clone, Copy, PartialEq)]
enum Case {
    Hit,
    Repair,
    BadReference,
    Stale,
    Forbidden,
    Missing,
    Unsupported,
    Cancel,
}
pub(super) fn signed_response(origin: &str, body: &serde_json::Value) -> serde_json::Value {
    let mut response = response(origin);
    let mut grant: wire::Grant = serde_json::from_slice(
        &STANDARD
            .decode(response["grant"]["payload"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    grant.binding = serde_json::from_value(body["binding"].clone()).unwrap();
    let payload = serde_json::to_vec(&grant).unwrap();
    response["grant"]["payload"] = STANDARD.encode(&payload).into();
    response["grant"]["signature"] = STANDARD
        .encode(
            key()
                .sign(&[wire::DOMAIN, payload.as_slice()].concat())
                .as_ref(),
        )
        .into();
    response
}
fn package_response(response: &mut serde_json::Value, device_key: &str) {
    use crate::art_execution_grant::package as pkg;
    let grant = serde_json::from_slice(
        &STANDARD
            .decode(response["grant"]["payload"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let proof = pkg::visibility::Package {
        protocol_version: pkg::visibility::PROTOCOL.into(),
        package: pkg::Package {
            protocol_version: pkg::PROTOCOL.into(),
            grant,
            device_public_key: device_key.into(),
            resources: vec![],
        },
        ports: pkg::ports::Ports {
            inputs: vec![],
            outputs: vec![],
        },
        visibility: pkg::visibility::Visibility {
            inputs: vec![],
            outputs: vec![],
        },
    };
    let payload = serde_json::to_vec(&proof).unwrap();
    response["protocolVersion"] = pkg::visibility::PREPARE.into();
    response["package"] = serde_json::json!({"algorithm":"ed25519","keyId":"test-issuer",
        "payload":STANDARD.encode(&payload),"signature":STANDARD.encode(key().sign(&[pkg::visibility::DOMAIN,&payload].concat()).as_ref())});
}
fn exercise(case: Case) {
    let (listener, client, tls) = fixture();
    let origin = client.origin.clone();
    let (mut device, first_lease) = lease(&origin);
    let next_lease = Arc::new(device.lease(&origin, now_ms()).unwrap());
    let root = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-art-source-cache-1")
        .join(format!("http-{:?}", unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()));
    std::fs::create_dir(&root).unwrap();
    let store = crate::device_identity_store::Store::new(&root).unwrap();
    let mut loaded = store.load_or_create().unwrap();
    store.bind_and_save(&mut loaded, "device-1").unwrap();
    let device_key = loaded.identity().public_key().unwrap();
    drop(loaded);
    let cache = crate::art_source_cache::Store::new(&root);
    let peer_origin = origin.clone();
    let cancel = next_lease.clone();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for i in 0..2 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let headers = head.to_ascii_lowercase();
            assert!(headers.contains("authorization: device fixture-secret\r\n"));
            assert!(nonces.insert(
                headers
                    .lines()
                    .find(|s| s.starts_with("x-loom-device-nonce: "))
                    .unwrap()
                    .to_owned()
            ));
            assert!(head.starts_with("POST /v3/art/local/prepare-package "));
            assert_eq!(body["protocolVersion"], "loom.art.local.prepare-package.v3");
            if i == 0 {
                assert_eq!(body["knownSourceDigest"], "0".repeat(64));
            } else {
                let expected = if matches!(case, Case::Repair | Case::BadReference) {
                    "corrupt;"
                } else {
                    SOURCE
                };
                assert_eq!(
                    body["knownSourceDigest"],
                    format!("{:x}", Sha256::digest(expected))
                );
            }
            if i > 0 {
                assert_eq!(body["binding"]["requestId"], "request:second");
            }
            let mut data = signed_response(&peer_origin, &body);
            if i == 1 {
                if case == Case::Unsupported {
                    reply(&mut stream, 404, b"{}");
                    listener.set_nonblocking(true).unwrap();
                    let deadline = Instant::now() + Duration::from_millis(200);
                    while Instant::now() < deadline {
                        assert!(
                            matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
                        );
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    continue;
                }
                if case == Case::Forbidden {
                    reply(&mut stream, 403, b"{}");
                    continue;
                }
                if case == Case::Stale {
                    data = response(&peer_origin);
                }
                data["source"] = serde_json::Value::Null;
                if case == Case::Repair {
                    data["source"] = STANDARD.encode(SOURCE).into();
                }
                if case == Case::Missing {
                    data.as_object_mut().unwrap().remove("source");
                }
                if case == Case::Cancel {
                    cancel.revoke();
                    continue; // 原租约取消会主动断开TLS，不要求写入已关闭的测试连接。
                }
            }
            package_response(&mut data, &device_key);
            reply(&mut stream, 200, &serde_json::to_vec(&data).unwrap());
        }
    });
    let first =
        Prepared::fetch_resumable(&client, first_lease, input(&origin), pin(), &cache).unwrap();
    drop(first);
    let file = std::fs::read_dir(root.join("art-source-cache"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|ext| ext == "js"))
        .unwrap();
    if matches!(case, Case::Repair | Case::BadReference) {
        std::fs::write(&file, b"corrupt;").unwrap();
    }
    let mut next = input(&origin);
    next.binding.request_id = "request:second".into();
    let result = Prepared::fetch_resumable(
        &client,
        next_lease,
        next,
        pin(),
        &crate::art_source_cache::Store::new(&root),
    );
    peer.join().unwrap();
    match case {
        Case::Hit | Case::Repair => {
            let prepared = result.unwrap();
            assert_eq!(prepared.binding().request_id, "request:second");
            let (mut local, frame) = prepared
                .mount(
                    view(&origin),
                    crate::surface_javascript::fixture::snapshot(1, 0),
                    true,
                    &|| false,
                )
                .unwrap();
            assert_eq!(
                frame.scene.root().children()[0].props()["text"],
                "downloaded"
            );
            local.stop();
            assert_eq!(std::fs::read(&file).unwrap(), SOURCE.as_bytes());
        }
        Case::BadReference => assert!(matches!(
            result,
            Err(ArtError::Grant(crate::art_execution_grant::Error::Source))
        )),
        Case::Stale => assert!(matches!(
            result,
            Err(ArtError::Grant(crate::art_execution_grant::Error::Binding))
        )),
        Case::Forbidden => assert!(matches!(
            result,
            Err(ArtError::Http(crate::live_relay_http::Error::Status(403)))
        )),
        Case::Unsupported => assert!(matches!(
            result,
            Err(ArtError::Http(crate::live_relay_http::Error::Status(404)))
        )),
        Case::Missing => assert!(matches!(result, Err(ArtError::Wire))),
        Case::Cancel => assert!(result.is_err()),
    }
}
#[test]
fn cache_hit_and_repair_use_current_signed_grant_and_real_child() {
    exercise(Case::Hit);
    exercise(Case::Repair);
}
#[test]
fn cache_never_executes_stale_missing_or_wrong_source_reference() {
    exercise(Case::Stale);
    exercise(Case::Missing);
    exercise(Case::BadReference);
}
#[test]
fn current_cache_rejects_unsupported_route_without_replay() {
    exercise(Case::Unsupported);
    exercise(Case::Forbidden);
}
#[test]
fn cache_cancelled_request_cannot_mount_saved_code() {
    exercise(Case::Cancel);
}
