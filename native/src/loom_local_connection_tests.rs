//! Local management channel and own-public-key authorization boundaries; no real user profiles.
use super::*;
#[test]
fn local_loom_renewal_preserves_inflight_lease_but_explicit_revoke_cancels_both() {
    use crate::device_session_client::tests::session;
    let mut owner = crate::device_authorization_owner::Owner::new();
    let origin = "https://localhost:443";
    owner.install(origin, session(origin, 10_000), 0).unwrap();
    let old = owner.lease(origin, 0).unwrap();
    owner.install(origin, session(origin, 20_000), 0).unwrap();
    let new = owner.lease(origin, 0).unwrap();
    assert!(!old.is_revoked());
    assert!(old.validate(origin, 10_000).is_err());
    assert_eq!(new.expires_at_ms(), 20_000);
    owner.invalidate(origin).unwrap();
    assert!(old.is_revoked() && new.is_revoked());
}
#[test]
fn local_loom_never_accepts_remote_administrator_target() {
    for origin in [
        "http://192.168.1.2:8765",
        "https://127.0.0.1:8765",
        "http://example.test",
    ] {
        assert!(matches!(
            http::Local::new(crate::loom_media_target::Target::new(
                origin.into(),
                Some("secret".into())
            )),
            Err(Error::LocalOnly)
        ));
    }
    assert!(http::Local::new(crate::loom_media_target::Target::new(
        "http://localhost:8765".into(),
        Some("secret".into())
    ))
    .is_ok());
}
fn registry(approval: &str, enabled: bool, duplicate: bool) -> Value {
    let device = json!({"id":"device:own","publicKey":"own","approval":approval,"enabled":enabled});
    json!({"devices":[device.clone()],"pending":if duplicate { vec![device] } else { vec![] }})
}
#[test]
fn local_loom_does_not_revive_revoked_or_ambiguous_identity() {
    for (approval, enabled, duplicate) in [
        ("rejected", true, false),
        ("approved", false, false),
        ("approved", true, true),
    ] {
        assert!(matches!(
            own_device(registry(approval, enabled, duplicate), "own"),
            Err(Error::DeviceDenied)
        ));
    }
    assert!(own_device(registry("approved", true, false), "other")
        .unwrap()
        .is_none());
    assert_eq!(
        own_device(registry("approved", true, false), "own")
            .unwrap()
            .unwrap()
            .approval,
        "approved"
    );
}

#[test]
fn loom_connection_remote_first_registration_and_reconnect_verify_once_each() {
    use crate::device_session_client::tests::{
        accept, fixture_with_host, pairing::Files, reply, request,
    };
    use crate::loom_remote_connection::Document;
    let (files, _, loaded) = Files::new();
    let profile = files.root();
    let directory = profile.to_owned();
    let public = loaded.identity().public_key().unwrap();
    let (listener, client, config, certificate_pem) = fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    let id = format!("loom-{}", "a".repeat(64));
    let expected = id.clone();
    let peer = std::thread::spawn(move || {
        let exchange = |path: &str, status, response: Value| {
            let mut stream = rustls::StreamOwned::new(
                rustls::ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert_eq!(head.lines().next().unwrap(), format!("{path} HTTP/1.1"));
            if path.ends_with("/connections/request") {
                let saved = Document::load(&directory).unwrap().unwrap();
                assert_eq!(saved.loom_id, expected);
                assert!(saved.device_id.is_none());
                assert_eq!(body["publicKey"], public);
                assert_eq!(body["password"], "fixture-code");
            }
            if path.ends_with("/connection-info") {
                assert!(head
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("Authorization: Device fixture-token")));
            }
            reply(&mut stream, status, &serde_json::to_vec(&response).unwrap());
        };
        for reconnect in [false, true] {
            exchange(
                "GET /v1/otter/identity",
                200,
                json!({"protocolVersion":"loom.connection.v1","loomId":expected}),
            );
            if !reconnect {
                exchange(
                    "POST /v1/otter/connections/request",
                    201,
                    json!({"connection":{"deviceId":"device-fixture"}}),
                );
            }
            exchange(
                "POST /v1/device-sessions/challenges",
                200,
                json!({"deviceId":"device-fixture","challengeId":"fixture","challenge":"value"}),
            );
            exchange(
                "POST /v1/device-sessions",
                200,
                json!({"deviceId":"device-fixture","token":"fixture-token","expiresAtMs":u64::MAX}),
            );
            let reported_id = if reconnect {
                format!("loom-{}", "b".repeat(64))
            } else {
                expected.clone()
            };
            exchange(
                "GET /v1/otter/connection-info",
                200,
                json!({"loom":{"loomId":reported_id},"account":"Fixture"}),
            );
        }
    });
    let document = Document {
        schema_version: 1,
        origin,
        loom_id: id.clone(),
        certificate_pem,
        device_id: None,
    };
    let cancel = Cancel::new();
    let connected = document
        .verify(&cancel)
        .unwrap()
        .connect_new(profile, &cancel, "fixture-code")
        .unwrap();
    assert_eq!(connected.loom_id, id);
    assert_eq!(connected.name, "Fixture");
    assert_eq!(connected.session.device_id(), "device-fixture");
    let saved = Document::load(profile).unwrap().unwrap();
    assert_eq!(saved.device_id.as_deref(), Some("device-fixture"));
    assert_eq!(saved.connect(profile, &cancel).err(), Some(Error::Response));
    peer.join().unwrap();
}

#[test]
fn loom_connection_local_rejects_unbound_tls_identity_before_touching_profile() {
    use crate::device_session_client::tests::{accept, fixture_with_host, reply, request};
    let (listener, client, config, pem) = fixture_with_host("127.0.0.1");
    let certificate = pem.lines().nth(1).unwrap();
    let origin = client.origin();
    let id = format!("loom-{}", "a".repeat(64));
    let local_info = json!({"protocolVersion":"loom.connection.v1","loomId":id,
        "endpoint":origin,"certificate":certificate,"discoverable":true});
    let peer = std::thread::spawn(move || {
        let mut stream = rustls::StreamOwned::new(
            rustls::ServerConnection::new(config).unwrap(),
            accept(&listener),
        );
        let (head, _) = request(&mut stream);
        assert_eq!(head.lines().next(), Some("GET /v1/otter/identity HTTP/1.1"));
        let body = json!({"protocolVersion":"loom.connection.v1","loomId":id,
            "endpoint":"https://127.0.0.1:1"});
        reply(&mut stream, 200, &serde_json::to_vec(&body).unwrap());
    });
    let local = http::Local::new(crate::loom_media_target::Target::new(
        "http://127.0.0.1:1".into(),
        Some("fixture-local".into()),
    ))
    .unwrap();
    assert_eq!(
        connect_discovered_local(
            local,
            local_info,
            Path::new("invalid-relative-profile"),
            &Cancel::new()
        )
        .err(),
        Some(Error::Tls)
    );
    peer.join().unwrap();
}
