//! 真实 Application/隐藏窗口、后台读取与显式确认；不自动操纵用户信任配置。
use super::*;
use crate::art_issuer_trust::{Document as Trust, Entry};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;
const ORIGIN: &str = "https://loom.example";
fn trust() -> Trust {
    Trust {
        issuers: vec![Entry {
            origin: ORIGIN.into(),
            key_id: "issuer:1".into(),
            public_key: STANDARD.encode([7; 32]),
        }],
    }
}
fn bytes(trust: Trust) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schemaVersion":2,"artIssuerTrust":trust})).unwrap()
}
fn lease() -> (
    crate::device_authorization_lease::Grant,
    Arc<crate::device_authorization_lease::Lease>,
) {
    use crate::{
        device_authorization_lease::Grant, device_identity::Identity,
        device_session_attempt::Attempt,
    };
    let mut identity = Identity::generate().unwrap();
    identity.bind("device:1").unwrap();
    let session = Attempt::new(&identity, ORIGIN)
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"device:1","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            br#"{"deviceId":"device:1","token":"test-only","expiresAtMs":9007199254740991}"#,
            1,
        )
        .unwrap();
    let mut owner = Grant::new(session);
    let lease = Arc::new(owner.lease(ORIGIN, 1).unwrap());
    (owner, lease)
}
unsafe fn startup(app: &mut Application, fixture: &Fixture) -> Result<()> {
    app.install_startup_settings(
        crate::settings_source::Source::new(fixture.0.clone(), fs::read(&fixture.0).unwrap())
            .unwrap(),
    )
}
unsafe fn import(app: &mut Application, fixture: &Fixture, accept: bool) -> Result<usize> {
    queue(app);
    let mut confirmations = 0;
    app.pending_naming_import_confirm_with(
        |_, _| Ok(Some(fixture.0.clone())),
        |_, _, _| panic!("not loaded yet"),
    )?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.naming_import_job.is_some() {
        app.pending_naming_import_confirm_with(
            |_, _| panic!("unexpected file dialog"),
            |_, before, next| {
                assert_ne!(before, next);
                confirmations += 1;
                Ok(accept)
            },
        )?;
        assert!(Instant::now() < deadline, "issuer import deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(confirmations)
}
#[test]
fn startup_installs_exact_trust_and_application_drop_revokes_it() {
    unsafe {
        let _environment = environment();
        let fixture = Fixture::new(&bytes(trust()));
        let mut app = app();
        let (_owner, lease) = lease();
        assert!(app.art_issuers.authorize(ORIGIN, lease.clone()).is_err());
        startup(&mut app, &fixture).unwrap();
        let _authorization = app.art_issuers.authorize(ORIGIN, lease.clone()).unwrap();
        assert!(!lease.is_revoked());
        drop(app);
        assert!(lease.is_revoked());
    }
}
#[test]
fn naming_import_cannot_silently_add_trust_and_decline_preserves_source() {
    unsafe {
        let _environment = environment();
        let fixture = Fixture::new(&bytes(trust()));
        let mut app = app();
        let before = app.naming_document.clone();
        assert!(import(&mut app, &fixture, false).is_err());
        assert!(Arc::ptr_eq(&before, &app.naming_document));
        assert!(app.naming_source.is_none());
        let (_owner, lease) = lease();
        assert!(app.art_issuers.authorize(ORIGIN, lease).is_err());
        assert_eq!(fs::read(&fixture.0).unwrap(), bytes(trust()));
    }
}
#[test]
fn confirmed_import_then_removal_revoke_original_lease_without_extra_confirmation() {
    unsafe {
        let _environment = environment();
        let fixture = Fixture::new(&bytes(trust()));
        let mut app = app();
        assert_eq!(import(&mut app, &fixture, true).unwrap(), 1);
        let (_owner, lease) = lease();
        let _authorization = app.art_issuers.authorize(ORIGIN, lease.clone()).unwrap();
        assert_eq!(import(&mut app, &fixture, false).unwrap(), 0);
        assert!(!lease.is_revoked());
        let removed = Fixture::new(&bytes(Trust::default()));
        assert!(import(&mut app, &removed, false).is_err());
        assert!(!lease.is_revoked());
        assert_eq!(import(&mut app, &removed, true).unwrap(), 1);
        assert!(lease.is_revoked());
    }
}
#[test]
fn malformed_startup_or_import_never_replaces_existing_trust() {
    unsafe {
        let _environment = environment();
        let valid = Fixture::new(&bytes(trust()));
        let mut app = app();
        startup(&mut app, &valid).unwrap();
        let (_owner, lease) = lease();
        let _authorization = app.art_issuers.authorize(ORIGIN, lease.clone()).unwrap();
        let before = app.naming_document.clone();
        let bad=Fixture::new(br#"{"artIssuerTrust":{"issuers":[{"origin":"https://loom.example","keyId":"x","publicKey":"broken"}]}}"#);
        assert!(startup(&mut app, &bad).is_err());
        assert!(Arc::ptr_eq(&before, &app.naming_document));
        assert!(import(&mut app, &bad, true).is_err());
        assert!(Arc::ptr_eq(&before, &app.naming_document));
        assert!(!lease.is_revoked());
    }
}
#[test]
fn confirmation_lists_independent_key_fingerprint_and_removal() {
    let added =
        crate::desktop::application::art_issuer_confirmation::describe(&Trust::default(), &trust())
            .unwrap();
    assert!(added.contains(ORIGIN));
    assert!(added.contains("issuer:1"));
    assert!(added.contains("SHA-256:"));
    assert!(added.contains("独立可信渠道"));
    let removed =
        crate::desktop::application::art_issuer_confirmation::describe(&trust(), &Trust::default())
            .unwrap();
    assert!(removed.contains("移除旧信任"));
}
