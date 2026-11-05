//! 实际Loom LAN目录/控制/源WSS/原生窗口精确帧、撤权和关闭重启；非普通双产品/物理跨端。
use super::*;
#[path = "live_lan_joint_peer_tests.rs"]
mod peer;
use crate::{
    byte_budget::Budget,
    device_authorization_config::Config,
    live_relay_control_runtime::{LocalArtTarget, Runtime},
};

fn wait_file(root: &std::path::Path, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !root.join(name).exists() {
        assert!(Instant::now() < deadline, "joint phase deadline: {name}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
#[ignore = "explicit real Loom LAN Live media and native pixel fixture"]
fn live_lan_joint_real_loom() {
    let input = PathBuf::from(std::env::var_os("OTTER_LOCAL_ART_JOINT_INPUT").unwrap());
    assert!(input.is_absolute() && fs::metadata(&input).unwrap().len() <= 64 * 1024);
    let data: serde_json::Value =
        serde_json::from_slice(&zeroize::Zeroizing::new(fs::read(&input).unwrap())).unwrap();
    let root = input.parent().unwrap();
    let identity =
        crate::device_identity::Identity::parse(&serde_json::to_vec(&data["identity"]).unwrap())
            .unwrap();
    let binding: wire::Binding =
        serde_json::from_value(data["request"]["binding"].clone()).unwrap();
    let url = url::Url::parse(&binding.origin).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let mut roots = RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            STANDARD
                .decode(data["certificate"].as_str().unwrap())
                .unwrap(),
        ))
        .unwrap();
    let client = Client::new(
        &binding.origin,
        roots.clone(),
        vec![format!("127.0.0.1:{}", url.port().unwrap())
            .parse()
            .unwrap()],
    )
    .unwrap();
    let create_body = serde_json::to_vec(&serde_json::json!({"envelope":data["start"]})).unwrap();
    let (status, response) = client
        .exchange(
            crate::device_session_client::Request {
                method: "POST",
                path: "/v1/live/sessions",
                body: &create_body,
                device: Some((
                    data["sourceToken"].as_str().unwrap(),
                    "00000000000000000000000000000001",
                )),
                limit: 64 * 1024,
            },
            &AtomicBool::new(false),
            &mut |_| {},
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
    assert_eq!(status, 201, "{}", String::from_utf8_lossy(&response));
    let session = client
        .issue_once(&identity, &AtomicBool::new(false), |_| {})
        .unwrap();
    let mut owner = DeviceGrant::new(session);
    let lease = Arc::new(owner.lease(&binding.origin, now_ms()).unwrap());
    let pin = Pin {
        key_id: data["issuer"]["keyId"].as_str().unwrap().into(),
        public_key: STANDARD
            .decode(data["issuer"]["publicKey"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap(),
    };
    let request = ArtRequest {
        binding: binding.clone(),
        permissions: serde_json::from_value(data["request"]["permissions"].clone()).unwrap(),
        services: vec![],
    };
    let prepared = Prepared::fetch(&client, lease.clone(), request, pin).unwrap();
    let control = lease.control(&client).unwrap();
    let catalog = control.catalog().unwrap();
    assert_eq!(catalog.len(), 1);
    assert!(!catalog[0].member);
    assert_eq!(catalog[0].identity.session(), "live:lan/part");
    let proof = prepared.viewer_proof().unwrap();
    let pem = root.join("lan-ca.pem");
    fs::write(
        &pem,
        format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            data["certificate"].as_str().unwrap()
        ),
    )
    .unwrap();
    let profile = root.join("native-live-profile");
    fs::create_dir(&profile).unwrap();
    let config = Config::from_lookup(&binding.origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(profile.clone().into_os_string()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(binding.origin.clone().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(pem.clone().into_os_string()),
        _ => None,
    })
    .unwrap();
    let budget = Budget::new(4 * 1024 * 1024);
    let runtime = Runtime::start_local_art_viewer(
        config,
        owner.lease(&binding.origin, now_ms()).unwrap(),
        LocalArtTarget {
            session: "live:lan/part".into(),
            selected: catalog[0].identity.clone(),
            proof,
        },
        budget.clone(),
        |events| {
            if events.iter().all(|event| event.kind() == "session_state") {
                Ok(())
            } else {
                Err(())
            }
        },
    )
    .unwrap();
    let _com = peer::Apartment::new();
    let mut native = crate::desktop::composition::relay::session::Session::from_runtime(runtime);
    native.show().unwrap();
    let source_token = data["sourceToken"].as_str().unwrap();
    let viewer_token = data["viewerToken"].as_str().unwrap();
    let mut source = peer::connect(
        &binding.origin,
        roots.clone(),
        source_token,
        "source",
        "lan-source-1",
    )
    .unwrap();
    source
        .send(tungstenite::Message::Binary(peer::frame()))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(outcome) = native.poll() {
            panic!(
                "native LAN viewer ended early: {:?}; {:?}",
                outcome.exit, outcome.window_error
            );
        }
        if let Some((metadata, pixel)) = native.test_frame().unwrap() {
            assert_eq!(
                (metadata.width, metadata.height, metadata.frame_id),
                (2, 2, 1)
            );
            assert_eq!(pixel, [17, 83, 211, 255]);
            break;
        }
        assert!(Instant::now() < deadline, "LAN native pixel deadline");
        peer::pump();
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut viewers = Vec::new();
    for index in 0..4 {
        let mut socket = peer::connect(
            &binding.origin,
            roots.clone(),
            viewer_token,
            "viewer",
            &format!("lan-viewer-{index}"),
        )
        .unwrap();
        peer::binary(&mut socket);
        viewers.push(socket);
    }
    assert!(
        matches!(peer::connect(&binding.origin, roots.clone(), source_token, "source", "lan-source-1"), Err(error) if matches!(error.as_ref(), tungstenite::Error::Http(response) if response.status().as_u16() == 409))
    );
    // More than the four short HTTP workers now own media. Directory/read/events must still complete.
    assert!(control.catalog().unwrap()[0].member);
    control
        .snapshot("live:lan/part")
        .unwrap()
        .renewal(&catalog[0].identity, &binding.device_id)
        .unwrap();
    write_once(&root.join("lan-ready.json"), br#"{"httpsControl":true,"sourceWss":true,"viewers":5,"nativePixel":[17,83,211,255],"nonceReplayRejected":true}"#);
    wait_file(root, "lan-viewer-revoked");
    for viewer in &mut viewers {
        peer::closed(viewer, true);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(outcome) = native.poll() {
            assert!(outcome.window_error.is_none());
            assert!(outcome.error_message().is_some());
            break;
        }
        assert!(Instant::now() < deadline, "native revoked cleanup deadline");
        peer::pump();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!native.has_window());
    drop(native);
    assert_eq!(budget.used(), 0);
    write_once(
        &root.join("lan-revoked.json"),
        br#"{"policyClosed":true,"nativeReclaimed":true,"encodedBytes":0}"#,
    );
    wait_file(root, "lan-disabled");
    peer::closed(&mut source, false);
    write_once(
        &root.join("lan-stopped.json"),
        br#"{"oldSourceClosed":true}"#,
    );
    wait_file(root, "lan-restarted.json");
    let restarted: String =
        serde_json::from_slice(&fs::read(root.join("lan-restarted.json")).unwrap()).unwrap();
    let mut resumed = peer::connect(
        &restarted,
        roots.clone(),
        source_token,
        "source",
        "lan-source-restart",
    )
    .unwrap();
    assert!(
        matches!(peer::connect(&restarted, roots, viewer_token, "viewer", "lan-revoked-restart"), Err(error) if matches!(error.as_ref(), tungstenite::Error::Http(response) if response.status().as_u16() == 401))
    );
    write_once(
        &root.join("lan-restart-ready.json"),
        br#"{"newSourceConnected":true,"disabledViewerRejected":true}"#,
    );
    wait_file(root, "lan-final-stop");
    peer::closed(&mut resumed, false);
    write_once(
        &root.join("lan-complete.json"),
        br#"{"fullMediaTested":true,"physicalCrossDevice":false,"ordinaryDualProduct":false}"#,
    );
}
