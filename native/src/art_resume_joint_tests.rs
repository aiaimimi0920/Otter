//! 真Loom签发后跨两个Otter测试进程；第二进程没有Device Session，服务端已退出。
use super::*;
use crate::{
    art_issuer_trust::{Authority, Document, Entry},
    art_local_requests::{description, Registry},
};
#[test]
#[ignore = "only the real Loom cross-process resume fixture launches this consumer"]
fn art_resume_joint_real_loom() {
    let _serial = js::SERIAL.lock().unwrap();
    let input = PathBuf::from(std::env::var_os("OTTER_LOCAL_ART_JOINT_INPUT").unwrap());
    let data: serde_json::Value =
        serde_json::from_slice(&zeroize::Zeroizing::new(fs::read(&input).unwrap())).unwrap();
    let root = input.parent().unwrap();
    let online = std::env::var("OTTER_ART_RESUME_STAGE").unwrap() == "online";
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
    let lease = if online {
        fs::write(
            root.join("device-identity.json"),
            identity.protected_document().unwrap(),
        )
        .unwrap();
        let url = url::Url::parse(&binding.origin).unwrap();
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
        let request = make_request(&binding, &data);
        let authorization = authority.authorize(&binding.origin, lease.clone()).unwrap();
        let prepared = authorization
            .prepare_cached(&client, request, &crate::art_source_cache::Store::new(root))
            .unwrap();
        assert!(prepared.online());
        drop(prepared);
        lease
    } else {
        Arc::new(authority.pending_local(&binding.origin).unwrap())
    };
    if !online {
        let workspace = "workspace:offline-process";
        let mut registry = Registry::new(workspace.into());
        let context = description::Request {
            origin: binding.origin.clone(),
            workspace: workspace.into(),
            instance: "instance:offline-process".into(),
            art: binding.art_id.clone(),
        };
        let pending = registry
            .begin_description(
                crate::workspace_id::UnitId::parse("unit:offline").unwrap(),
                context,
                lease.clone(),
                &mut authority,
            )
            .unwrap();
        let prepared = registry.accept(pending.fetch_offline(root)).unwrap();
        assert!(!prepared.online());
        let current = prepared.binding().clone();
        assert_ne!(current.request_id, binding.request_id);
        assert_ne!(current.workspace_id, binding.workspace_id);
        assert_ne!(current.instance_id, binding.instance_id);
        let mount = json!({"instance":{"descriptor":{"instanceId":current.instance_id,"generation":current.generation},"attachments":{"attachment:offline":{"descriptor":{"attachmentId":"attachment:offline","instanceId":current.instance_id,"hookNodeId":"node:offline","deviceId":current.device_id},"lifecycle":"active","lifecycleRevision":1}}}});
        let view = crate::surface_live_binding::Binding::from_mount(
            &serde_json::to_vec(&mount).unwrap(),
            &current.origin,
            &current.device_id,
            "node:offline",
        )
        .unwrap();
        let (mut local, frame) = prepared
            .mount(view, js::snapshot(1, 1), false, &|| false)
            .unwrap();
        assert_eq!(js::text(&frame.scene), "offline-answer");
        assert!(matches!(&frame.result.as_ref().unwrap().outputs["answer"],
            crate::surface_javascript_wire::result::ResolvedOutput::Value(value) if *value == json!(42)));
        local.refresh(&current, false, &|| false).unwrap();
        local.stop();
        write_once(&root.join("otter-resume-offline.json"),b"{\"sourceVerified\":true,\"freshHostBinding\":true,\"formalResult\":true,\"deviceSession\":false,\"offlineService\":true}");
    } else {
        write_once(
            &root.join("otter-resume-online.json"),
            b"{\"resumablePrepared\":true}",
        );
    }
}
fn make_request(binding: &wire::Binding, data: &serde_json::Value) -> ArtRequest {
    ArtRequest {
        binding: binding.clone(),
        permissions: serde_json::from_value(data["request"]["permissions"].clone()).unwrap(),
        services: serde_json::from_value(data["request"]["services"].clone()).unwrap(),
    }
}
