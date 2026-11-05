//! Application 测试的真实回环 TLS 服务；仅签发测试源码，临时身份/CA 保留在 linshi。
use crate::device_session_client::tests::{accept, fixture_with_host, reply, request};
use crate::{
    art_execution_grant::wire,
    device_authorization_config::Config,
    device_authorization_owner::{Event, Owner},
    device_authorization_runtime::Task,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair};
use rustls::{ServerConnection, StreamOwned};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
// 只有首条 mount 已携带宿主视口时才成功；不能靠后续 Animation 修补首帧。
const SOURCE: &str = "NeuroSurface.define({mount({root}){root.style.boxSizing='border-box';root.style.width='100%';if(!(root.getBoundingClientRect().width>0&&devicePixelRatio>0))throw Error('missing mount viewport');let s=document.createElement('span');s.textContent='application-local';root.replaceChildren(s)}});";
pub(super) struct Peer {
    pub origin: String,
    root: PathBuf,
    client: crate::device_session_client::Client,
    server: JoinHandle<()>,
}
fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[47; 32]).unwrap()
}
impl Peer {
    pub fn new() -> Self {
        Self::with_source(SOURCE, false)
    }
    pub(super) fn with_source(source: &'static str, results: bool) -> Self {
        Self::with_art_source("vendor/application", source, results)
    }
    pub(super) fn with_art_source(art: &'static str, source: &'static str, results: bool) -> Self {
        Self::with_rounds(art, source, results, 1, None)
    }
    pub(super) fn with_cached_source(source: &'static str) -> Self {
        Self::with_rounds("vendor/application", source, true, 2, None)
    }
    pub(super) fn with_input(source: &'static str, input: &str) -> Self {
        use crate::art_execution_grant::package::ports::{Port, Ports};
        Self::with_rounds(
            "vendor/application",
            source,
            true,
            1,
            Some((
                Ports {
                    inputs: vec![Port {
                        name: input.into(),
                        kind: "image".into(),
                    }],
                    outputs: vec![],
                },
                None,
            )),
        )
    }
    pub(super) fn with_ports(
        source: &'static str,
        ports: crate::art_execution_grant::package::ports::Ports,
    ) -> Self {
        Self::with_rounds("vendor/application", source, true, 1, Some((ports, None)))
    }
    pub(super) fn with_visibility(
        source: &'static str,
        ports: crate::art_execution_grant::package::ports::Ports,
        visibility: crate::art_execution_grant::package::visibility::Visibility,
    ) -> Self {
        Self::with_rounds(
            "vendor/application",
            source,
            true,
            1,
            Some((ports, Some(visibility))),
        )
    }
    fn with_rounds(
        art: &'static str,
        source: &'static str,
        results: bool,
        rounds: usize,
        ports: Option<(
            crate::art_execution_grant::package::ports::Ports,
            Option<crate::art_execution_grant::package::visibility::Visibility>,
        )>,
    ) -> Self {
        // listener 仅绑 IPv4；配置重解析不能先对无 listener 的 ::1 各等两秒。
        let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
        let origin = client.origin().to_owned();
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-v450-local-application",
        )
        .join(format!(
            "application-peer-{:?}",
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("ca.pem"), pem).unwrap();
        let store = crate::device_identity_store::Store::new(&root).unwrap();
        let mut loaded = store.load_or_create().unwrap();
        store
            .bind_and_save(&mut loaded, "device-application")
            .unwrap();
        let device_key = loaded.identity().public_key().unwrap();
        let expected_origin = origin.clone();
        let mut permissions = vec![wire::Permission::LocalState, wire::Permission::Preview];
        if results {
            permissions.push(wire::Permission::WorkspaceResult);
        }
        use crate::art_execution_grant::package::{
            ports::{Port, Ports},
            visibility::Visibility,
        };
        let (ports, visibility) = ports.unwrap_or_else(|| {
            (
                Ports {
                    inputs: vec![Port {
                        name: "input_image".into(),
                        kind: "image".into(),
                    }],
                    outputs: vec![Port {
                        name: "output_image".into(),
                        kind: "image".into(),
                    }],
                },
                None,
            )
        });
        let visibility = visibility.unwrap_or_else(|| Visibility {
            inputs: vec![true; ports.inputs.len()],
            outputs: vec![true; ports.outputs.len()],
        });
        let server = std::thread::spawn(move || {
            let mut nonce = String::new();
            let mut previous: Option<wire::Binding> = None;
            for index in 0..4 * rounds {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                let data = match index % 4 {
                    0 => {
                        assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                        serde_json::json!({"deviceId":"device-application","challengeId":"id","challenge":"value"})
                    }
                    1 => {
                        assert!(head.starts_with("POST /v1/device-sessions "));
                        serde_json::json!({"deviceId":"device-application","token":"application-test-only","expiresAtMs":9007199254740991u64})
                    }
                    2 => {
                        assert!(head.starts_with("POST /v1/art/local/describe "));
                        assert_eq!(body, serde_json::json!({"artId":art}));
                        nonce = device_headers(&head);
                        serde_json::json!({"protocolVersion":"loom.art.local.describe.v1","descriptor":{
                            "artId":art,"artVersion":"1.0.0","packageDigest":"b".repeat(64),
                            "sdkVersion":wire::SDK,"permissions":permissions,"services":[]}})
                    }
                    _ => {
                        let cached = index >= 4;
                        assert!(head.starts_with("POST /v3/art/local/prepare-package "));
                        assert_eq!(
                            body["protocolVersion"],
                            crate::art_execution_grant::package::visibility::PREPARE
                        );
                        assert_ne!(device_headers(&head), nonce);
                        let current: wire::Binding =
                            serde_json::from_value(body["binding"].clone()).unwrap();
                        if let Some(old) = previous.as_ref() {
                            assert_ne!(current.request_id, old.request_id);
                            assert_ne!(current.workspace_id, old.workspace_id);
                            assert_ne!(current.instance_id, old.instance_id);
                        }
                        previous = Some(current);
                        if cached {
                            assert_eq!(
                                body["protocolVersion"],
                                crate::art_execution_grant::package::visibility::PREPARE
                            );
                            assert_eq!(
                                body["knownSourceDigest"],
                                format!("{:x}", Sha256::digest(source))
                            );
                        }
                        let mut data = prepare(body, &expected_origin, art, source, &permissions);
                        let grant: wire::Grant = serde_json::from_slice(
                            &STANDARD
                                .decode(data["grant"]["payload"].as_str().unwrap())
                                .unwrap(),
                        )
                        .unwrap();
                        let ticket = crate::art_execution_grant::package::Package {
                            protocol_version: crate::art_execution_grant::package::PROTOCOL.into(),
                            device_public_key: device_key.clone(),
                            grant,
                            resources: Vec::new(),
                        };
                        let proof = crate::art_execution_grant::package::visibility::Package {
                            protocol_version:
                                crate::art_execution_grant::package::visibility::PROTOCOL.into(),
                            package: ticket,
                            ports: ports.clone(),
                            visibility: visibility.clone(),
                        };
                        let payload = serde_json::to_vec(&proof).unwrap();
                        let domain = crate::art_execution_grant::package::visibility::DOMAIN;
                        let protocol = crate::art_execution_grant::package::visibility::PREPARE;
                        let signature = key().sign(&[domain, payload.as_slice()].concat());
                        data["package"] = serde_json::json!({"algorithm":"ed25519","keyId":"application-issuer","payload":STANDARD.encode(payload),"signature":STANDARD.encode(signature.as_ref())});
                        data["protocolVersion"] = protocol.into();
                        if cached {
                            data["source"] = serde_json::Value::Null;
                        }
                        data
                    }
                };
                reply(&mut stream, 200, &serde_json::to_vec(&data).unwrap());
            }
        });
        Self {
            origin,
            root,
            client,
            server,
        }
    }
    pub fn config(&self) -> Config {
        Config::from_lookup(&self.origin, |key| match key {
            "OTTER_NATIVE_PROFILE_DIR" => Some(self.root.clone().into_os_string()),
            "OTTER_LOOM_TLS_ORIGIN" => Some(self.origin.clone().into()),
            "OTTER_LOOM_TLS_CA_FILE" => Some(self.root.join("ca.pem").into_os_string()),
            _ => None,
        })
        .unwrap()
    }
    pub fn issuer(&self) -> crate::art_issuer_trust::Entry {
        crate::art_issuer_trust::Entry {
            origin: self.origin.clone(),
            key_id: "application-issuer".into(),
            public_key: STANDARD.encode(key().public_key().as_ref()),
        }
    }
    pub fn authority(&self) -> crate::art_issuer_trust::Authority {
        crate::art_issuer_trust::Authority::new(crate::art_issuer_trust::Document {
            issuers: vec![self.issuer()],
        })
        .unwrap()
    }
    pub fn authorize(&self, owner: &mut Owner) {
        owner
            .start(Task::Renew {
                client: self.client.clone(),
                store: crate::device_identity_store::Store::new(&self.root).unwrap(),
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let events = owner.poll();
            if !events.is_empty() {
                assert!(
                    matches!(&events[0], Event::Authorized { origin, .. } if origin == &self.origin)
                );
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    pub fn finish(self) {
        self.server.join().unwrap();
    }
}
fn device_headers(head: &str) -> String {
    let head = head.to_ascii_lowercase();
    assert!(head.contains("authorization: device application-test-only\r\n"));
    head.lines()
        .find(|line| line.starts_with("x-loom-device-nonce: "))
        .unwrap()
        .into()
}
fn prepare(
    body: serde_json::Value,
    origin: &str,
    art: &str,
    source: &str,
    permissions: &[wire::Permission],
) -> serde_json::Value {
    assert_eq!(body["permissions"], serde_json::json!(permissions));
    let binding: wire::Binding = serde_json::from_value(body["binding"].clone()).unwrap();
    assert_eq!(binding.origin, origin);
    assert_eq!(binding.device_id, "device-application");
    assert_eq!(binding.art_id, art);
    assert_eq!(binding.art_version, "1.0.0");
    assert_eq!(binding.package_digest, "b".repeat(64));
    assert_eq!(binding.generation, 1);
    for id in [&binding.workspace_id, &binding.instance_id] {
        assert!(uuid::Uuid::parse_str(id).is_ok());
    }
    assert!(binding.request_id.starts_with("request:"));
    assert!(uuid::Uuid::parse_str(binding.request_id.strip_prefix("request:").unwrap()).is_ok());
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let digest = format!("{:x}", Sha256::digest(source));
    let grant = wire::Grant {
        protocol_version: wire::PROTOCOL.into(),
        grant_id: "grant:application".into(),
        binding,
        execution_owner: "otter".into(),
        result_owner: "otter_workspace".into(),
        permissions: permissions.to_vec(),
        services: vec![],
        issued_at_ms: time - 1,
        expires_at_ms: time + 60000,
        offline_until_ms: time + 30000,
        source: wire::Source {
            resource_id: format!("sha256:{digest}"),
            digest,
            size: source.len() as u64,
        },
    };
    let payload = serde_json::to_vec(&grant).unwrap();
    let signature = key().sign(&[wire::DOMAIN, payload.as_slice()].concat());
    serde_json::json!({"protocolVersion":"loom.art.local.prepare.v1","source":STANDARD.encode(source),
        "grant":{"algorithm":"ed25519","keyId":"application-issuer","payload":STANDARD.encode(payload),"signature":STANDARD.encode(signature.as_ref())}})
}
