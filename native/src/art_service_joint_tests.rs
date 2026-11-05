//! 显式跨仓测试入口：使用真实 Loom 挑战/签名会话、下载/验签与服务路由，不使用模拟回复。
use super::*;
use crate::surface_javascript::fixture as js;
use serde_json::json;
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};
#[path = "art_service_joint_confirmation.rs"]
mod confirmation;
#[path = "art_package_joint_tests.rs"]
mod package_joint;
#[path = "art_resume_joint_tests.rs"]
mod resume_joint;
#[path = "art_live_viewer_joint_tests.rs"]
mod viewer_joint;

fn write_once(path: &std::path::Path, bytes: &[u8]) {
    let staging = path.with_extension("pending");
    assert!(!path.exists());
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
    drop(file);
    fs::rename(staging, path).unwrap();
}
#[test]
#[ignore = "launched only by the live Loom cross-repository fixture"]
fn art_service_joint_real_loom() {
    let _serial = js::SERIAL.lock().unwrap();
    let input = PathBuf::from(
        std::env::var_os("OTTER_LOCAL_ART_JOINT_INPUT").expect("explicit fixture input"),
    );
    assert!(input.is_absolute());
    assert!(fs::metadata(&input).unwrap().len() <= 64 * 1024);
    let bytes = zeroize::Zeroizing::new(fs::read(&input).unwrap());
    let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let identity =
        crate::device_identity::Identity::parse(&serde_json::to_vec(&data["identity"]).unwrap())
            .unwrap();
    let mut binding: wire::Binding =
        serde_json::from_value(data["request"]["binding"].clone()).unwrap();
    let url = url::Url::parse(&binding.origin).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let address = format!("127.0.0.1:{}", url.port().unwrap())
        .parse()
        .unwrap();
    let mut roots = RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            STANDARD
                .decode(data["certificate"].as_str().unwrap())
                .unwrap(),
        ))
        .unwrap();
    let client = Client::new(&binding.origin, roots, vec![address]).unwrap();
    let session = client
        .issue_once(&identity, &AtomicBool::new(false), |_| {})
        .unwrap();
    let mut owner = DeviceGrant::new(session);
    let lease = Arc::new(owner.lease(&binding.origin, now_ms()).unwrap());
    let pin = || Pin {
        key_id: data["issuer"]["keyId"].as_str().unwrap().to_owned(),
        public_key: STANDARD
            .decode(data["issuer"]["publicKey"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap(),
    };
    let root = input.parent().unwrap();
    fs::write(
        root.join("device-identity.json"),
        identity.protected_document().unwrap(),
    )
    .unwrap();
    let cache = crate::art_source_cache::Store::new(root);
    let make_request = |binding: wire::Binding| ArtRequest {
        binding,
        permissions: serde_json::from_value(data["request"]["permissions"].clone()).unwrap(),
        services: serde_json::from_value(data["request"]["services"].clone()).unwrap(),
    };
    let first = Prepared::fetch_resumable(
        &client,
        lease.clone(),
        make_request(binding.clone()),
        pin(),
        &cache,
    )
    .unwrap();
    assert_eq!(first.binding(), &binding);
    drop(first);
    let original_request = binding.request_id.clone();
    binding.request_id.push_str(":cached");
    let prepared = Prepared::fetch_resumable(
        &client,
        lease.clone(),
        make_request(binding.clone()),
        pin(),
        &cache,
    )
    .unwrap();
    assert_eq!(prepared.binding(), &binding);
    let sources: Vec<_> = fs::read_dir(root.join("art-source-cache"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    assert_eq!(sources.len(), 1);
    let source_digest = format!("{:x}", Sha256::digest(fs::read(&sources[0]).unwrap()));
    write_once(&root.join("otter-joint-cache.json"), &serde_json::to_vec(&json!({"sourceDigest":source_digest,"originalRequest":original_request,"currentRequest":binding.request_id,"cachedPrepare":true})).unwrap());
    let mount = json!({"instance":{"descriptor":{"instanceId":binding.instance_id,"generation":binding.generation},
        "attachments":{"attachment:1":{"descriptor":{"attachmentId":"attachment:1","instanceId":binding.instance_id,
        "hookNodeId":"node:1","deviceId":binding.device_id},"lifecycle":"active","lifecycleRevision":1}}}});
    let view = crate::surface_live_binding::Binding::from_mount(
        &serde_json::to_vec(&mount).unwrap(),
        &binding.origin,
        &binding.device_id,
        "node:1",
    )
    .unwrap();
    let images = data["imageService"].as_bool().unwrap_or(false);
    let mut confirmation = data["confirmationService"]
        .as_bool()
        .unwrap_or(false)
        .then(confirmation::Host::new);
    let (mut local, frame) = if images {
        let image = crate::surface_javascript_wire::images::tests::resource();
        let expected = image
            .png
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let result = prepared
            .mount_content_observed(
                view,
                crate::surface_javascript::signed::runtime::Content {
                    snapshot: js::snapshot(1, 1),
                    resources: json!({}),
                    images: vec![image],
                },
                true,
                None,
                None,
                &|| false,
            )
            .unwrap();
        assert_eq!(js::text(&result.1.scene), format!("image:{expected}"));
        result
    } else {
        let result = prepared
            .mount_content_observed(
                view,
                crate::surface_javascript::signed::runtime::Content::empty(js::snapshot(1, 1)),
                true,
                None,
                confirmation.as_ref().map(|host| host.channel.clone()),
                &|| false,
            )
            .unwrap();
        if data["canvasService"].as_bool().unwrap_or(false) {
            let text = js::text(&result.1.scene);
            let hex = text.strip_prefix("image:").unwrap();
            assert_eq!(hex.len() % 2, 0);
            let png: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                .read_info()
                .unwrap();
            assert_eq!([reader.info().width, reader.info().height], [2, 1]);
            let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
            reader.next_frame(&mut pixels).unwrap();
            assert_eq!(pixels, [255, 0, 0, 255, 0, 0, 255, 128]);
        } else {
            assert_eq!(js::text(&result.1.scene), "calls:1");
            if data["imageOutputService"].as_bool().unwrap_or(false) {
                assert_eq!(
                    result.1.images.values().next().unwrap().pixels.as_ref(),
                    [0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128]
                );
            }
        }
        result
    };
    let target = frame.scene.root().children()[1].id().to_owned();
    let frame = local.click(&binding, true, &target, &|| false).unwrap();
    assert_eq!(js::text(&frame.scene), "null-complete");
    if let Some(host) = &mut confirmation {
        host.finish();
    }
    write_once(
        &root.join("otter-joint-ready.json"),
        b"{\"calls\":1,\"null\":true}",
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    while !root.join("otter-joint-revoked").exists() {
        assert!(Instant::now() < deadline, "fixture revocation not received");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(local.click(&binding, true, &target, &|| false).is_err());
    assert!(local.refresh(&binding, true, &|| false).is_err());
    binding.request_id.push_str(":revoked");
    assert!(Prepared::fetch_resumable(
        &client,
        lease.clone(),
        make_request(binding.clone()),
        pin(),
        &cache
    )
    .is_err());
    drop(local);
    drop(owner);
    assert!(lease.is_revoked());
    write_once(&root.join("otter-joint-complete"), b"revocation-rejected");
}
