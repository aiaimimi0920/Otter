//! Real Loom HTTPS issuer/prepare/native-viewer attach; deliberately not a media or visible-product gate.
use super::*;
#[path = "live_lan_joint_tests.rs"]
mod lan;
#[test]
#[ignore = "explicit real Loom native Art viewer admission fixture"]
fn art_live_viewer_joint_real_loom() {
    let input =
        PathBuf::from(std::env::var_os("OTTER_LOCAL_ART_JOINT_INPUT").expect("explicit input"));
    assert!(input.is_absolute());
    assert!(fs::metadata(&input).unwrap().len() <= 64 * 1024);
    let bytes = zeroize::Zeroizing::new(fs::read(&input).unwrap());
    let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
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
        roots,
        vec![format!("127.0.0.1:{}", url.port().unwrap())
            .parse()
            .unwrap()],
    )
    .unwrap();
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
    let proof = prepared.viewer_proof().unwrap();
    let expected = crate::live_relay_viewer_identity::Snapshot::parse(
        &serde_json::to_vec(&data["snapshot"]).unwrap(),
        "live:art-joint",
    )
    .unwrap();
    let control = lease.control(&client).unwrap();
    let (joined, _) = control
        .attach_local_art(expected.identity(), &proof, 1, false)
        .unwrap();
    joined
        .renewal(expected.identity(), &binding.device_id)
        .unwrap();
    control
        .change_local_art_controller(expected.identity(), &proof, 2, true)
        .unwrap();
    control
        .forward_local_art_input(
            expected.identity(),
            &proof,
            crate::live_relay_http::RequesterCursor {
                control: 3,
                input: 1,
            },
            crate::live_input::Event::Key {
                code: 65,
                down: true,
            },
        )
        .unwrap();
    let root = input.parent().unwrap();
    write_once(
        &root.join("viewer-ready.json"),
        br#"{"nativeProof":true,"member":true,"https":true,"nativeControl":true,"inputAccepted":true}"#,
    );
    let deadline = Instant::now() + Duration::from_secs(15);
    while !root.join("viewer-revoked").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(matches!(
        control.attach_local_art(expected.identity(), &proof, 2, true),
        Err(crate::live_relay_http::Error::Status(403))
    ));
    assert!(matches!(
        control.change_local_art_controller(expected.identity(), &proof, 4, true),
        Err(crate::live_relay_http::Error::Status(403))
    ));
    assert!(matches!(
        control.forward_local_art_input(
            expected.identity(),
            &proof,
            crate::live_relay_http::RequesterCursor {
                control: 4,
                input: 2
            },
            crate::live_input::Event::Key {
                code: 65,
                down: false
            }
        ),
        Err(crate::live_relay_http::Error::Status(403))
    ));
    // Reduction stays available after Art approval revoke, but still uses the original Device session.
    control
        .change_local_art_controller(expected.identity(), &proof, 4, false)
        .unwrap();
    drop(prepared);
    drop(owner);
    assert!(proof.validate().is_err());
    write_once(
        &root.join("viewer-complete.json"),
        br#"{"revocationRejected":true,"releaseAfterRevocation":true,"mediaTested":false,"sourceExecutionTested":false}"#,
    );
}
