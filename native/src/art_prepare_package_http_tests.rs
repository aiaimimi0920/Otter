//! 真实TLS包下载/缓存/取消负例；只用本测试profile与固定issuer，不接触用户配置。
use super::*;
use crate::{
    art_execution_grant::{package as pkg, resume::Ticket},
    art_source_cache::Store,
};
#[derive(Clone, Copy, PartialEq, Debug)]
enum Case {
    Hit,
    WrongProof,
    Tampered,
    WrongId,
    WrongBytes,
    Forbidden,
    Cancel,
    Full,
    WrongProtocol,
    Reject(u16),
}
const BYTES: &[u8] = b"package text";
fn root() -> std::path::PathBuf {
    let root=std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-art-package-resources-1").join(format!("tls-{:?}",unsafe {windows::Win32::System::Com::CoCreateGuid()}.unwrap()));
    std::fs::create_dir(&root).unwrap();
    root
}
fn sign(value: &impl serde::Serialize, domain: &[u8]) -> wire::Envelope {
    let payload = serde_json::to_vec(value).unwrap();
    wire::Envelope {
        algorithm: "ed25519".into(),
        key_id: "test-issuer".into(),
        payload: STANDARD.encode(&payload),
        signature: STANDARD.encode(key().sign(&[domain, &payload].concat()).as_ref()),
    }
}
fn exercise(case: Case) {
    let (listener, client, tls) = fixture();
    let origin = client.origin.clone();
    let root = root();
    let store = crate::device_identity_store::Store::new(&root).unwrap();
    let mut loaded = store.load_or_create().unwrap();
    store.bind_and_save(&mut loaded, "device-1").unwrap();
    let device_key = loaded.identity().public_key().unwrap();
    drop(loaded);
    let cache = Store::new(&root);
    if case == Case::Full {
        std::fs::create_dir(root.join("art-source-cache")).unwrap();
        let f = std::fs::File::create(root.join("art-source-cache/unknown.dat")).unwrap();
        f.set_len(32 * 1024 * 1024).unwrap();
    }
    let (mut owner, lease) = lease(&origin);
    let second = Arc::new(owner.lease(&origin, now_ms()).unwrap());
    let cancelled = lease.clone();
    let peer_origin = origin.clone();
    let peer_key = device_key.clone();
    let resource = pkg::Resource {
        id: "text".into(),
        mime_type: "text/plain".into(),
        size: BYTES.len() as u64,
        digest: format!("{:x}", Sha256::digest(BYTES)),
        dimensions: [0, 0],
    };
    let declaration = resource.clone();
    let peer = std::thread::spawn(move || {
        for round in 0..if case == Case::Hit { 2 } else { 1 } {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v3/art/local/prepare-package "));
            assert_eq!(body["protocolVersion"], pkg::visibility::PREPARE);
            if let Case::Reject(status) = case {
                reply(&mut stream, status, b"{}");
                listener.set_nonblocking(true).unwrap();
                let deadline = std::time::Instant::now() + Duration::from_millis(200);
                while std::time::Instant::now() < deadline {
                    assert!(
                        matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock),
                        "unexpected downgrade connection"
                    );
                    std::thread::sleep(Duration::from_millis(2));
                }
                continue;
            }
            let mut response = super::cache_tests::signed_response(&peer_origin, &body);
            let original: wire::Grant = serde_json::from_slice(
                &STANDARD
                    .decode(response["grant"]["payload"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap();
            let mut package = pkg::Package {
                protocol_version: pkg::PROTOCOL.into(),
                grant: original.clone(),
                device_public_key: peer_key.clone(),
                resources: vec![declaration.clone()],
            };
            if case == Case::WrongProof {
                package.grant.binding.request_id = "other".into();
            }
            let proof = pkg::visibility::Package {
                protocol_version: pkg::visibility::PROTOCOL.into(),
                package,
                ports: pkg::ports::Ports {
                    inputs: vec![],
                    outputs: vec![],
                },
                visibility: pkg::visibility::Visibility {
                    inputs: vec![],
                    outputs: vec![],
                },
            };
            let mut envelope = sign(&proof, pkg::visibility::DOMAIN);
            if case == Case::Tampered {
                envelope.payload = STANDARD.encode(b"{}");
            }
            response["package"] = serde_json::to_value(envelope).unwrap();
            response["protocolVersion"] = if case == Case::WrongProtocol {
                pkg::PREPARE
            } else {
                pkg::visibility::PREPARE
            }
            .into();
            if round == 1 {
                response["source"] = serde_json::Value::Null;
            }
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            if matches!(
                case,
                Case::WrongProof | Case::Tampered | Case::WrongProtocol
            ) || round == 1
            {
                continue;
            }
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/art/local/resource "));
            assert_eq!(body["resourceId"], "text");
            if case == Case::Forbidden {
                reply(&mut stream, 403, b"{}");
                continue;
            }
            if case == Case::Cancel {
                cancelled.revoke();
            }
            let data = serde_json::json!({"protocolVersion":pkg::RESOURCE,"grantId":original.grant_id,"resourceId":if case==Case::WrongId{"other"}else{"text"},"digest":declaration.digest,"bytes":STANDARD.encode(if case==Case::WrongBytes{b"changed text"}else{BYTES})});
            // 撤销会关闭客户端；写失败是本负例预期，不把服务端BrokenPipe当产品失败。
            if case != Case::Cancel {
                reply(&mut stream, 200, &serde_json::to_vec(&data).unwrap());
            }
        }
    });
    let prepared = Prepared::fetch_resumable(&client, lease, input(&origin), pin(), &cache);
    let success = matches!(case, Case::Hit | Case::Full);
    assert_eq!(
        prepared.is_ok(),
        success,
        "{case:?}: {:?}",
        prepared.as_ref().err()
    );
    if case == Case::Hit {
        let mut next = input(&origin);
        next.binding.request_id = "request:new".into();
        let again = Prepared::fetch_resumable(&client, second, next, pin(), &cache).unwrap();
        assert!(again.online());
    }
    peer.join().unwrap();
    let slot = crate::art_offline_store::Slot::open(
        &root,
        &origin,
        "vendor/download",
        &pin(),
        &device_key,
    )
    .unwrap();
    if case == Case::Hit {
        let bytes = slot.take(now_ms(), &|| false).unwrap();
        let ticket = Ticket::verify(
            &bytes,
            crate::art_execution_grant::Issuer {
                key_id: "test-issuer",
                public_key: &pin().public_key,
            },
        )
        .unwrap();
        assert_eq!(ticket.resources.len(), usize::from(case == Case::Hit));
    } else {
        assert!(slot.take(now_ms(), &|| false).is_err());
    }
    if case == Case::Hit {
        assert_eq!(
            crate::art_package_cache::load(
                &cache,
                &Prepared::source_key(&input(&origin), &pin()),
                &resource
            )
            .unwrap()
            .unwrap(),
            BYTES
        );
    }
}
#[test]
fn package_tls_cache_hit_still_gets_new_grant_without_refetching_assets() {
    exercise(Case::Hit)
}
#[test]
fn package_tls_rejects_mixed_original_grant() {
    exercise(Case::WrongProof)
}
#[test]
fn package_tls_rejects_tampered_proof() {
    exercise(Case::Tampered)
}
#[test]
fn package_tls_rejects_wrong_resource_identity() {
    exercise(Case::WrongId)
}
#[test]
fn package_tls_rejects_wrong_resource_digest_bytes() {
    exercise(Case::WrongBytes)
}
#[test]
fn package_tls_forbidden_never_publishes_offline_ticket() {
    exercise(Case::Forbidden)
}
#[test]
fn package_tls_cancelled_download_never_publishes_offline_ticket() {
    exercise(Case::Cancel)
}
#[test]
fn package_tls_full_shared_cache_keeps_online_but_no_offline_permission() {
    exercise(Case::Full)
}
#[test]
fn current_package_404_is_terminal_without_discovery_or_replay() {
    exercise(Case::Reject(404))
}
#[test]
fn current_package_rejects_non_current_response_protocol() {
    exercise(Case::WrongProtocol)
}
#[test]
fn current_package_rejection_is_terminal_without_discovery_or_replay() {
    for status in [400, 403] {
        exercise(Case::Reject(status));
    }
}
