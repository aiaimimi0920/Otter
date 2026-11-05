//! 真Loom资源准入和完整缓存离线消费；固定2x2像素oracle不依赖包资源转换实现。
use super::*;
use crate::{
    art_issuer_trust::{Authority, Document, Entry},
    art_local_requests::{description, Registry},
};
#[test]
#[ignore = "launched only by the real Loom package fixture"]
fn art_package_joint_real_loom() {
    let _serial = js::SERIAL.lock().unwrap();
    let input = PathBuf::from(std::env::var_os("OTTER_LOCAL_ART_JOINT_INPUT").unwrap());
    assert!(input.is_absolute());
    assert!(fs::metadata(&input).unwrap().len() <= 64 * 1024);
    let data: serde_json::Value =
        serde_json::from_slice(&zeroize::Zeroizing::new(fs::read(&input).unwrap())).unwrap();
    let root = input.parent().unwrap();
    let stage = std::env::var("OTTER_ART_RESUME_STAGE").unwrap();
    let online = stage == "online";
    let binding: wire::Binding =
        serde_json::from_value(data["request"]["binding"].clone()).unwrap();
    let identity =
        crate::device_identity::Identity::parse(&serde_json::to_vec(&data["identity"]).unwrap())
            .unwrap();
    let mut authority = Authority::new(Document {
        issuers: vec![Entry {
            origin: binding.origin.clone(),
            key_id: data["issuer"]["keyId"].as_str().unwrap().into(),
            public_key: data["issuer"]["publicKey"].as_str().unwrap().into(),
        }],
    })
    .unwrap();
    let mut owner;
    // Registry拥有原停止租约；必须活到挂载/正式结果消费结束，不能在准备分支末尾退休。
    let mut registry_owner = None;
    let prepared = if online {
        fs::write(
            root.join("device-identity.json"),
            identity.protected_document().unwrap(),
        )
        .unwrap();
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
        owner = DeviceGrant::new(
            client
                .issue_once(&identity, &AtomicBool::new(false), |_| {})
                .unwrap(),
        );
        let lease = Arc::new(owner.lease(&binding.origin, now_ms()).unwrap());
        authority
            .authorize(&binding.origin, lease)
            .unwrap()
            .prepare_cached(
                &client,
                ArtRequest {
                    binding: binding.clone(),
                    permissions: serde_json::from_value(data["request"]["permissions"].clone())
                        .unwrap(),
                    services: vec![],
                },
                &crate::art_source_cache::Store::new(root),
            )
            .unwrap()
    } else {
        let lease = Arc::new(authority.pending_local(&binding.origin).unwrap());
        let workspace = "workspace:package-offline";
        let registry = registry_owner.insert(Registry::new(workspace.into()));
        let context = description::Request {
            origin: binding.origin.clone(),
            workspace: workspace.into(),
            instance: "instance:package-offline".into(),
            art: binding.art_id.clone(),
        };
        let mut pending = Some(
            registry
                .begin_description(
                    crate::workspace_id::UnitId::parse("unit:package").unwrap(),
                    context,
                    lease.clone(),
                    &mut authority,
                )
                .unwrap(),
        );
        if stage == "incomplete" {
            let cache = root.join("art-source-cache");
            let path = fs::read_dir(&cache)
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| p.extension().is_some_and(|v| v == "blob"))
                .unwrap();
            let quarantine = path.with_extension("held-test");
            assert!(!quarantine.exists());
            fs::rename(&path, &quarantine).unwrap();
            let result = registry.accept(pending.take().unwrap().fetch_offline(root));
            fs::rename(quarantine, path).unwrap();
            assert!(result.is_err());
            let url = url::Url::parse(&binding.origin).unwrap();
            let address = format!("127.0.0.1:{}", url.port().unwrap())
                .parse()
                .unwrap();
            assert!(lease
                .control(
                    &Client::new(&binding.origin, RootCertStore::empty(), vec![address]).unwrap()
                )
                .is_err());
            write_once(
                &root.join("otter-package-incomplete.json"),
                b"{\"incompleteRejected\":true,\"deviceSession\":false}",
            );
            return;
        }
        registry
            .accept(pending.take().unwrap().fetch_offline(root))
            .unwrap()
    };
    assert_eq!(prepared.online(), online);
    assert_eq!(
        serde_json::to_value(prepared.ports.as_ref().expect("signed ports required")).unwrap(),
        data["expectedPorts"]
    );
    assert_eq!(
        serde_json::to_value(
            prepared
                .visibility
                .as_ref()
                .expect("signed v3 visibility required")
        )
        .unwrap(),
        data["expectedVisibility"]
    );
    let current = prepared.binding().clone();
    if !online {
        assert_ne!(current.request_id, binding.request_id);
        assert_ne!(current.workspace_id, binding.workspace_id);
    }
    let mount = json!({"instance":{"descriptor":{"instanceId":current.instance_id,"generation":current.generation},"attachments":{"attachment:package":{"descriptor":{"attachmentId":"attachment:package","instanceId":current.instance_id,"hookNodeId":"node:package","deviceId":current.device_id},"lifecycle":"active","lifecycleRevision":1}}}});
    let view = crate::surface_live_binding::Binding::from_mount(
        &serde_json::to_vec(&mount).unwrap(),
        &current.origin,
        &current.device_id,
        "node:package",
    )
    .unwrap();
    let (mut local, frame) = prepared
        .mount(view, js::snapshot(1, 1), online, &|| false)
        .unwrap();
    let result = frame.result.unwrap();
    assert!(
        matches!(&result.outputs["answer"],crate::surface_javascript_wire::result::ResolvedOutput::Value(v) if *v==json!(42))
    );
    let crate::surface_javascript_wire::result::ResolvedOutput::Image(image) =
        &result.outputs["image"]
    else {
        panic!("formal package image missing")
    };
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(
        image.pixels.as_ref(),
        &[0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128]
    );
    local.refresh(&current, online, &|| false).unwrap();
    local.stop();
    write_once(
        &root.join(format!("otter-package-{stage}.json")),
        if online {
            b"{\"packageResources\":true,\"formalPixels\":true,\"formalResult\":true,\"signedPorts\":true,\"signedVisibility\":true,\"deviceSession\":true}"
        } else {
            b"{\"packageResources\":true,\"formalPixels\":true,\"formalResult\":true,\"signedPorts\":true,\"signedVisibility\":true,\"deviceSession\":false}"
        },
    );
}
