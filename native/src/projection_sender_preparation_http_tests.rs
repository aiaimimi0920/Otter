//! 后台配置准备与既有签名者核验；缺失/更换身份不自动生成，错误授权不建立连接。
use super::*;
#[test]
fn sender_configured_worker_reads_scoped_ca_and_dns_without_repairing_identity() {
    let (listener, client, tls, pem) = crate::device_session_client::tests::fixture_with_pem();
    let (directory, _store, request) = prepared(&client);
    let identity = directory.identity().join("device-identity.json");
    let before = std::fs::read(&identity).unwrap();
    let ca = directory.0.join("ca.pem");
    let config =
        crate::device_authorization_config::Config::from_lookup(client.origin(), |key| match key {
            "OTTER_NATIVE_PROFILE_DIR" => Some(directory.0.as_os_str().to_owned()),
            "OTTER_LOOM_TLS_ORIGIN" => Some(client.origin().into()),
            "OTTER_LOOM_TLS_CA_FILE" => Some(ca.as_os_str().to_owned()),
            _ => None,
        })
        .unwrap();
    assert!(!ca.exists());
    std::fs::write(&ca, pem).unwrap();
    let value = response(&request);
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        let (head, _) = http_request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/read "));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret\r\n"));
        reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
        listener
    });
    let mut grant = grant(&client);
    let mut job = Job::start(
        config,
        grant.lease(client.origin(), 0).unwrap(),
        Operation::Send(Box::new(request)),
    )
    .unwrap();
    wait(&job);
    assert!(matches!(job.try_join(), Some(Ok(Output::Sent(_)))));
    no_request(&peer.join().unwrap());
    assert_eq!(std::fs::read(identity).unwrap(), before);
}
#[test]
fn sender_missing_or_changed_identity_and_invalid_grant_never_connect_or_repair() {
    let (listener, client, _) = fixture();
    let (directory, _store, request) = prepared(&client);
    let unchanged = std::fs::read(directory.path()).unwrap();
    let missing = Directory::new();
    let changed = Directory::new();
    changed.install_identity();
    let before = std::fs::read(changed.identity().join("device-identity.json")).unwrap();
    let mut grant = grant(&client);
    for identity in [missing.identity(), changed.identity()] {
        let mut job = start(
            &client,
            &mut grant,
            SendRequest::new(request.intent().clone(), identity),
        );
        wait(&job);
        assert!(matches!(
            job.try_join(),
            Some(Err(JobError::Sender(sender::Error::Identity)))
        ));
        no_request(&listener);
    }
    assert!(!missing.identity().exists());
    assert_eq!(
        std::fs::read(changed.identity().join("device-identity.json")).unwrap(),
        before
    );
    for mut other in [
        Grant::new(session("https://other.invalid", u64::MAX)),
        Grant::new(session(client.origin(), 1)),
    ] {
        let origin = if other.token_for(client.origin(), 0).is_some() {
            client.origin()
        } else {
            "https://other.invalid"
        };
        let mut job = Job::start_prepared(
            client.clone(),
            other.lease(origin, 0).unwrap(),
            Operation::Send(Box::new(request.clone())),
        )
        .unwrap();
        wait(&job);
        assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
        no_request(&listener);
    }
    assert_eq!(std::fs::read(directory.path()).unwrap(), unchanged);
}
#[test]
fn sender_different_actor_or_tampered_durable_signature_never_connect() {
    use base64::Engine;
    let (listener, client, _) = fixture();
    let (directory, store, request) = prepared(&client);
    let mut actor = crate::device_identity::Identity::generate().unwrap();
    actor.bind("other-device").unwrap();
    let session = crate::device_session_attempt::Attempt::new(&actor, client.origin())
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"other-device","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            &serde_json::to_vec(
                &json!({"deviceId":"other-device","token":"fixture-secret","expiresAtMs":u64::MAX}),
            )
            .unwrap(),
            0,
        )
        .unwrap();
    let mut wrong_actor = Grant::new(session);
    let mut job = start(&client, &mut wrong_actor, request.clone());
    wait(&job);
    assert!(matches!(
        job.try_join(),
        Some(Err(JobError::Sender(sender::Error::Identity)))
    ));
    no_request(&listener);
    let mut value: Value =
        serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
    value["entries"][0]["envelope"]["signature"]["value"] =
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode([0; 64])
            .into();
    drop((job, request, store));
    let changed = serde_json::to_vec(&value).unwrap();
    std::fs::write(directory.path(), &changed).unwrap();
    let store = sender::Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let intent = store
        .pending(&AtomicBool::new(false))
        .unwrap()
        .pop()
        .unwrap();
    let mut grant = grant(&client);
    let mut job = start(
        &client,
        &mut grant,
        SendRequest::new(intent, directory.identity()),
    );
    wait(&job);
    assert!(matches!(
        job.try_join(),
        Some(Err(JobError::Sender(sender::Error::Identity)))
    ));
    no_request(&listener);
    assert_eq!(std::fs::read(directory.path()).unwrap(), changed);
}
