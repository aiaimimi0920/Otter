//! 真实回环 TLS 下载、独立 issuer 验签和 child 挂载；不冒充生产 Loom/UI 集成。
use super::*;
use crate::{
    art_execution_grant::wire,
    device_authorization_lease::{Grant as DeviceGrant, Lease},
    live_relay_http::art_prepare::{Error as ArtError, Pin, Prepared, Request as ArtRequest},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};
use std::sync::Arc;
#[path = "art_prepare_cache_http_tests.rs"]
mod cache_tests;
#[path = "art_service_joint_tests.rs"]
mod joint;
#[path = "art_prepare_package_http_tests.rs"]
mod package_tests;
#[path = "art_service_http_tests.rs"]
mod service_tests;
const SOURCE: &str = "NeuroSurface.define({mount({root}){let s=document.createElement('span');s.textContent='downloaded';root.replaceChildren(s)}});";
struct Owners {
    _device: DeviceGrant,
    _trust: crate::art_issuer_trust::Authority,
    _requests: crate::desktop::art_local_tasks::Tasks,
    binding: wire::Binding,
}
fn authority(origin: &str, pin: Pin) -> crate::art_issuer_trust::Authority {
    use crate::art_issuer_trust::{Authority, Document, Entry};
    Authority::new(Document {
        issuers: vec![Entry {
            origin: origin.into(),
            key_id: pin.key_id,
            public_key: STANDARD.encode(pin.public_key),
        }],
    })
    .unwrap()
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap()
}
fn pin() -> Pin {
    Pin {
        key_id: "test-issuer".into(),
        public_key: key().public_key().as_ref().try_into().unwrap(),
    }
}
fn input(origin: &str) -> ArtRequest {
    ArtRequest {
        binding: wire::Binding {
            origin: origin.into(),
            request_id: "request:download".into(),
            device_id: "device-1".into(),
            workspace_id: "workspace:1".into(),
            instance_id: "instance:1".into(),
            generation: 1,
            art_id: "vendor/download".into(),
            art_version: "1.0.0".into(),
            package_digest: "a".repeat(64),
            sdk_version: wire::SDK.into(),
        },
        permissions: vec![wire::Permission::LocalState, wire::Permission::Preview],
        services: vec![],
    }
}
fn response(origin: &str) -> serde_json::Value {
    let r = input(origin);
    let time = now_ms();
    let digest = format!("{:x}", Sha256::digest(SOURCE));
    let g = wire::Grant {
        protocol_version: wire::PROTOCOL.into(),
        grant_id: "grant:download".into(),
        binding: r.binding,
        execution_owner: "otter".into(),
        result_owner: "otter_workspace".into(),
        permissions: r.permissions,
        services: r.services,
        issued_at_ms: time - 1,
        expires_at_ms: time + 60_000,
        offline_until_ms: time + 30_000,
        source: wire::Source {
            resource_id: format!("sha256:{digest}"),
            digest,
            size: SOURCE.len() as u64,
        },
    };
    let payload = serde_json::to_vec(&g).unwrap();
    let signature = key().sign(&[wire::DOMAIN, payload.as_slice()].concat());
    serde_json::json!({"protocolVersion":"loom.art.local.prepare.v1","source":STANDARD.encode(SOURCE),
        "grant":{"algorithm":"ed25519","keyId":"test-issuer","payload":STANDARD.encode(payload),"signature":STANDARD.encode(signature.as_ref())}})
}
fn lease(origin: &str) -> (DeviceGrant, Arc<Lease>) {
    let mut owner = DeviceGrant::new(session(origin, now_ms() + 60_000));
    let lease = Arc::new(owner.lease(origin, now_ms()).unwrap());
    (owner, lease)
}
fn view(origin: &str) -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:1","generation":1},"attachments":{"attachment:1":{"descriptor":{"attachmentId":"attachment:1","instanceId":"instance:1","hookNodeId":"node:1","deviceId":"device-1"},"lifecycle":"active","lifecycleRevision":1}}}}"#,origin,"device-1","node:1").unwrap()
}
fn fetch_case(
    change: impl FnOnce(&mut serde_json::Value),
    status: u16,
    wrong_pin: bool,
) -> (Result<Prepared, ArtError>, Owners, Arc<Lease>, String) {
    let (listener, client, config) = fixture();
    let origin = client.origin.clone();
    let (owner, lease) = lease(&origin);
    let mut trust = pin();
    if wrong_pin {
        trust.public_key = [0; 32];
    }
    let mut authority = authority(&origin, trust);
    let describe_listener = listener.try_clone().unwrap();
    let describe_config = config.clone();
    let describe_peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(describe_config).unwrap(),
            accept(&describe_listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/art/local/describe HTTP/1.1"));
        assert_eq!(body, serde_json::json!({"artId":"vendor/download"}));
        let header = head.to_ascii_lowercase();
        assert!(header.contains("authorization: device fixture-secret\r\n"));
        let nonce = header
            .lines()
            .find(|line| line.starts_with("x-loom-device-nonce: "))
            .unwrap()
            .to_owned();
        let descriptor = serde_json::json!({"protocolVersion":"loom.art.local.describe.v1","descriptor":{
            "artId":"vendor/download","artVersion":"1.0.0","packageDigest":"a".repeat(64),
            "sdkVersion":"otter.art.v1","permissions":["local_state","preview"],"services":[]}});
        reply(&mut stream, 200, &serde_json::to_vec(&descriptor).unwrap());
        nonce
    });
    let (completed, notified) = std::sync::mpsc::channel();
    let mut tasks = crate::desktop::art_local_tasks::Tasks::new(
        "workspace:1".into(),
        Arc::new(move || {
            let _ = completed.send(());
        }),
    );
    let token = tasks
        .start_description_with(
            crate::workspace_id::UnitId::parse("node:1").unwrap(),
            crate::art_local_requests::description::Request {
                origin: origin.clone(),
                workspace: "workspace:1".into(),
                instance: "instance:1".into(),
                art: "vendor/download".into(),
            },
            lease.clone(),
            &mut authority,
            move |pending| Ok(pending.fetch(&client)),
        )
        .unwrap();
    let described_nonce = describe_peer.join().unwrap();
    let mut binding = input(&origin).binding;
    binding.request_id = token.id.to_string();
    binding.generation = token.generation;
    let mut data = response(&origin);
    let mut grant: wire::Grant = serde_json::from_slice(
        &STANDARD
            .decode(data["grant"]["payload"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    grant.binding = binding.clone();
    let payload = serde_json::to_vec(&grant).unwrap();
    data["grant"]["signature"] = STANDARD
        .encode(
            key()
                .sign(&[wire::DOMAIN, payload.as_slice()].concat())
                .as_ref(),
        )
        .into();
    data["grant"]["payload"] = STANDARD.encode(payload).into();
    change(&mut data);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/art/local/prepare HTTP/1.1"));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret\r\n"));
        assert!(head.to_ascii_lowercase().contains("x-loom-device-nonce: "));
        assert!(!head.to_ascii_lowercase().contains(&described_nonce));
        assert_eq!(body["binding"]["artId"], "vendor/download");
        assert!(body.get("issuer").is_none());
        reply(&mut stream, status, &serde_json::to_vec(&data).unwrap());
    });
    notified.recv_timeout(Duration::from_secs(5)).unwrap();
    let result = tasks.take_settled().unwrap().result.map_err(|e| match e {
        crate::art_local_requests::Error::Download(crate::art_issuer_trust::Error::Prepare(e)) => e,
        _ => ArtError::Input,
    });
    peer.join().unwrap();
    (
        result,
        Owners {
            _device: owner,
            _trust: authority,
            _requests: tasks,
            binding,
        },
        lease,
        origin,
    )
}
#[test]
fn authenticated_source_download_mounts_real_child_and_keeps_original_lease() {
    let (result, owner, lease, origin) = fetch_case(|_| {}, 200, false);
    let (mut local, frame) = result
        .unwrap()
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
    let current = owner.binding.clone();
    drop(owner);
    assert!(lease.is_revoked());
    assert!(local.refresh(&current, true, &|| false).is_err());
}
#[test]
fn prepared_source_transfers_to_background_owner_and_revocation_reaps_idle_child() {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::{HANDLE, WAIT_OBJECT_0},
        System::Threading::WaitForMultipleObjects,
    };
    let (result, owner, lease, origin) = fetch_case(|_| {}, 200, false);
    let prepared = result.unwrap();
    assert_eq!(prepared.binding(), &owner.binding);
    let viewer = prepared.viewer_proof().unwrap();
    assert_eq!(viewer.binding(), &owner.binding);
    assert!(viewer.validate().is_ok());
    let mut runtime = crate::surface_javascript::signed::runtime::Runtime::start(
        prepared,
        view(&origin),
        crate::surface_javascript::fixture::snapshot(1, 0),
        true,
    )
    .unwrap();
    let handles: Vec<_> = runtime
        .wait_handles()
        .map(|handle| HANDLE(handle.as_raw_handle()))
        .collect();
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handles, false, 8000) }.0,
        WAIT_OBJECT_0.0 + 1
    );
    let frame = runtime.take_reply(&owner.binding).unwrap().unwrap();
    assert_eq!(
        frame.scene.root().children()[0].props()["text"],
        "downloaded"
    );
    drop(owner);
    assert!(lease.is_revoked());
    assert!(viewer.validate().is_err());
    let handles: Vec<_> = runtime
        .wait_handles()
        .map(|handle| HANDLE(handle.as_raw_handle()))
        .collect();
    assert_eq!(handles.len(), 1);
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handles, false, 8000) },
        WAIT_OBJECT_0
    );
    assert!(runtime.take_exit().unwrap().is_err());
    assert!(runtime.finished());
}
#[test]
fn response_tampering_untrusted_issuer_and_http_denial_never_prepare() {
    assert!(matches!(
        fetch_case(
            |v| v["source"] = STANDARD.encode("tampered").into(),
            200,
            false
        )
        .0,
        Err(ArtError::Grant(_))
    ));
    assert!(matches!(
        fetch_case(|_| {}, 200, true).0,
        Err(ArtError::Grant(_))
    ));
    assert!(matches!(
        fetch_case(|v| v["publicKey"] = "peer-selected".into(), 200, false).0,
        Err(ArtError::Wire)
    ));
    assert!(matches!(
        fetch_case(|_| {}, 403, false).0,
        Err(ArtError::Http(LiveError::Status(403)))
    ));
}
#[test]
fn revoked_prepared_result_and_cancelled_mount_do_not_start_child() {
    let (result, owner, _, origin) = fetch_case(|_| {}, 200, false);
    drop(owner);
    assert!(matches!(
        result.unwrap().mount(
            view(&origin),
            crate::surface_javascript::fixture::snapshot(1, 0),
            true,
            &|| false
        ),
        Err(ArtError::Grant(_))
    ));
    let (result, _owner, _, origin) = fetch_case(|_| {}, 200, false);
    assert!(matches!(
        result.unwrap().mount(
            view(&origin),
            crate::surface_javascript::fixture::snapshot(1, 0),
            true,
            &|| true
        ),
        Err(ArtError::Grant(_))
    ));
}
#[test]
fn wrong_origin_device_or_revoked_lease_are_rejected_before_network() {
    let (_listener, client, _config) = fixture();
    let (_owner, lease) = lease(&client.origin);
    let mut r = input(&client.origin);
    r.binding.device_id = "other".into();
    assert!(matches!(
        Prepared::fetch(&client, lease.clone(), r, pin()),
        Err(ArtError::Input)
    ));
    let r = input("https://other.example");
    assert!(matches!(
        Prepared::fetch(&client, lease.clone(), r, pin()),
        Err(ArtError::Input)
    ));
    lease.revoke();
    assert!(matches!(
        Prepared::fetch(&client, lease, input(&client.origin), pin()),
        Err(ArtError::Lease)
    ));
}
#[test]
fn revoking_original_lease_interrupts_inflight_download() {
    let (listener, client, config) = fixture();
    let origin = client.origin.clone();
    let (_owner, lease) = lease(&origin);
    let mut trust = authority(&origin, pin());
    let (completed, notified) = std::sync::mpsc::channel();
    let mut tasks = crate::desktop::art_local_tasks::Tasks::new(
        "workspace:1".into(),
        Arc::new(move || {
            let _ = completed.send(());
        }),
    );
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        ready_tx.send(()).unwrap();
        let mut byte = [0];
        assert!(!matches!(stream.read(&mut byte),Ok(n) if n>0));
    });
    tasks
        .start_client(
            crate::workspace_id::UnitId::parse("node:1").unwrap(),
            input(&origin),
            lease.clone(),
            &mut trust,
            client,
        )
        .unwrap();
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    trust
        .replace(crate::art_issuer_trust::Document::default())
        .unwrap();
    assert!(lease.is_revoked());
    notified.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(tasks.take_settled().is_none());
    assert!(!tasks.busy());
    peer.join().unwrap();
}

#[test]
fn revoking_original_lease_interrupts_inflight_description() {
    let (listener, client, config) = fixture();
    let (_owner, lease) = lease(&client.origin);
    let mut trust = authority(&client.origin, pin());
    let (done, notified) = std::sync::mpsc::channel();
    let mut tasks = crate::desktop::art_local_tasks::Tasks::new(
        "workspace:1".into(),
        Arc::new(move || {
            let _ = done.send(());
        }),
    );
    let origin = client.origin.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/art/local/describe HTTP/1.1"));
        ready_tx.send(()).unwrap();
        let mut byte = [0];
        assert!(!matches!(stream.read(&mut byte), Ok(n) if n > 0));
    });
    let unit = crate::workspace_id::UnitId::parse("node:1").unwrap();
    tasks
        .start_description_with(
            unit.clone(),
            crate::art_local_requests::description::Request {
                origin,
                workspace: "workspace:1".into(),
                instance: "instance:1".into(),
                art: "vendor/download".into(),
            },
            lease.clone(),
            &mut trust,
            move |pending| Ok(pending.fetch(&client)),
        )
        .unwrap();
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    tasks.cancel(&unit);
    assert!(lease.is_revoked());
    notified.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(tasks.take_settled().is_none());
    assert!(!tasks.busy());
    peer.join().unwrap();
}
