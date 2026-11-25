//! Synthetic signatures, PNGs, and fixed local HTTP; no user credentials, and fixtures are not OAuth/public-network projection.
use super::*;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use ring::signature::KeyPair;
use serde_json::{json, Value};
const SOURCE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const RECEIVER: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const CENTRAL: &str = "https://account.invalid";
#[path = "projection_account_http_tests.rs"]
mod http_tests;
#[path = "projection_account_joint_tests.rs"]
mod joint;
#[path = "projection_account_journal_tests.rs"]
mod journal_tests;
#[path = "projection_account_scene_tests.rs"]
pub(crate) mod scene_tests;
fn context(device: &str) -> Context {
    Context::parse(&serde_json::to_vec(&context_json(device)).unwrap()).unwrap()
}
fn context_json(device: &str) -> Value {
    json!({"status":"signed_in", "protocol":"neuro.loom-account.v1", "projectionProtocol":PROTOCOL,
        "origin":CENTRAL,"accountId":"account-1","deviceId":device,"deviceName":"Fixture",
        "expiresAtMs":now().unwrap()+120_000,"policy":{"serverOrigin":CENTRAL}})
}
fn png(color: u8) -> crate::pixels::Pixels {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[color, 83, 211, 255, 7, 8, 9, 255])
            .unwrap();
    }
    out.into()
}
fn image(color: u8) -> Image {
    Image::from_png(&png(color), 2, 1, &AtomicBool::new(false)).unwrap()
}
fn envelope(image: &Image) -> Envelope {
    let key = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[17; 32]).unwrap();
    let mut value = json!({"protocol":PROTOCOL,"projectionId":format!("projection:{}", "a".repeat(32)),
        "serverOrigin":CENTRAL,"source":{"deviceId":SOURCE,"accountId":"account-1",
        "publicKey":STANDARD.encode(key.public_key().as_ref()),"sessionId":"source-epoch","unitId":"unit-1","revision":1},
        "content":{"kind":"sticker","digest":image.digest()},"expiresAtMs":now().unwrap()+120_000,
        "nonce":"b".repeat(32),"signature":{"algorithm":"ed25519","keyId":SOURCE,"value":""}});
    let mut envelope: Envelope = serde_json::from_value(value.clone()).unwrap();
    // message is private to production modules; fixtures independently assemble the contract so signing/verifying cannot share a wrong sequence.
    let signed = [
        PROTOCOL.to_owned(),
        envelope.projection_id().into(),
        CENTRAL.into(),
        SOURCE.into(),
        "account-1".into(),
        value["source"]["publicKey"].as_str().unwrap().into(),
        "source-epoch".into(),
        "unit-1".into(),
        "1".into(),
        "sticker".into(),
        image.digest().into(),
        envelope.expires_at_ms().to_string(),
        "b".repeat(32),
        "ed25519".into(),
        SOURCE.into(),
    ]
    .join("\n");
    value["signature"]["value"] = URL_SAFE_NO_PAD
        .encode(key.sign(signed.as_bytes()).as_ref())
        .into();
    envelope = Envelope::parse(&value.to_string()).unwrap();
    envelope
}
fn wire(
    envelope: &Envelope,
    image: Option<&Image>,
    revision: u64,
    digest: &str,
    receiver: Option<&str>,
) -> Value {
    json!({"envelope":envelope,"revision":revision,"digest":digest,"linked":true,
        "receiverDeviceId":receiver,"receiverUnitId":receiver.map(|_| "receive-1"),
        "snapshot":image,"transport":"direct","error":null})
}
fn parsed(value: &Value, operation: &Operation<'_>, context: &Context) -> Result<State, Error> {
    response::parse(
        &serde_json::to_vec(value).unwrap(),
        operation,
        context,
        &AtomicBool::new(false),
    )
}
#[test]
fn signed_invitation_roundtrip_rejects_tampering_v1_unknown_fields_and_noncanonical_encoding() {
    let image = image(17);
    let envelope = envelope(&image);
    assert_eq!(
        Envelope::parse(&envelope.link().unwrap()).unwrap(),
        envelope
    );
    let original = serde_json::to_value(&envelope).unwrap();
    for (field, replacement) in [
        ("protocol", json!("neuro.qr-projection.v1")),
        ("serverOrigin", json!("https://foreign.invalid")),
        ("nonce", json!("c".repeat(32))),
        ("unexpected", json!(true)),
    ] {
        let mut changed = original.clone();
        changed[field] = replacement;
        assert!(Envelope::parse(&changed.to_string()).is_err(), "{field}");
    }
    let mut changed = original;
    changed["source"]["accountId"] = "account-2".into();
    assert!(Envelope::parse(&changed.to_string()).is_err());
    assert!(Envelope::parse(&(envelope.link().unwrap() + "=")).is_err());
    assert!(Envelope::parse(&"x".repeat(8193)).is_err());
}
#[test]
fn local_operation_contract_binds_account_actor_unit_epoch_and_explicit_consent() {
    let image = image(17);
    let envelope = envelope(&image);
    let source = context(SOURCE);
    let receiver = context(RECEIVER);
    let create = Operation::Create {
        unit: "unit-1",
        kind: "sticker",
        image: &image,
    };
    create.validate(&source).unwrap();
    let body: Value = serde_json::from_slice(&create.body().unwrap()).unwrap();
    assert_eq!(body["kind"], "create");
    assert_eq!(body["unitId"], "unit-1");
    assert!(body.get("envelope").is_none());
    assert!(body.get("origin").is_none());
    for confirmed in [false, true] {
        assert_eq!(
            Operation::Accept {
                envelope: &envelope,
                unit: "receive-1",
                revision: 1,
                digest: image.digest(),
                confirmed
            }
            .validate(&receiver)
            .is_ok(),
            confirmed
        );
    }
    assert!(Operation::Inspect {
        envelope: &envelope
    }
    .validate(&source)
    .is_err());
    let update = Operation::Update {
        envelope: &envelope,
        prior_revision: 1,
        revision: 2,
        image: &image,
    };
    update.validate(&source).unwrap();
    assert!(update.validate(&receiver).is_err());
    let body: Value = serde_json::from_slice(&update.body().unwrap()).unwrap();
    assert_eq!(body["sourceSessionId"], "source-epoch");
    assert_eq!(body["projectionId"], envelope.projection_id());
    let mut foreign = receiver;
    foreign.account_id = "foreign".into();
    Operation::Inspect {
        envelope: &envelope,
    }
    .validate(&foreign)
    .unwrap();
    Operation::Accept {
        envelope: &envelope,
        unit: "receive-1",
        revision: 1,
        digest: image.digest(),
        confirmed: true,
    }
    .validate(&foreign)
    .unwrap();
    Operation::Read {
        envelope: &envelope,
        known_revision: 1,
        known_digest: image.digest(),
        receiver_unit: Some("receive-1"),
    }
    .validate(&foreign)
    .unwrap();
    let mut forged_source = source;
    forged_source.account_id = "wrong-source-account".into();
    assert!(update.validate(&forged_source).is_err());
    assert!(Operation::Read {
        envelope: &envelope,
        known_revision: 1,
        known_digest: image.digest(),
        receiver_unit: None
    }
    .validate(&forged_source)
    .is_err());
    foreign.origin = "https://wrong-origin.example".into();
    assert_eq!(
        Operation::Inspect {
            envelope: &envelope
        }
        .validate(&foreign),
        Err(Error::Identity)
    );
}
#[test]
fn response_verifies_real_png_and_never_equates_invited_with_accepted_or_displayed() {
    let image = image(17);
    let envelope = envelope(&image);
    let op = Operation::Inspect {
        envelope: &envelope,
    };
    let value = wire(&envelope, Some(&image), 1, image.digest(), None);
    let State::Content(content) = parsed(&value, &op, &context(RECEIVER)).unwrap() else {
        panic!()
    };
    assert!(content.linked);
    assert!(content.receiver_unit_id.is_none());
    assert_eq!(
        &*content.image.unwrap().png(&AtomicBool::new(false)).unwrap(),
        &*png(17)
    );
    for field in [
        "digest",
        "transport",
        "receiverDeviceId",
        "snapshot",
        "route",
    ] {
        let mut changed = value.clone();
        changed[field] = match field {
            "digest" => json!("0".repeat(64)),
            "transport" => json!("shared_loom"),
            "receiverDeviceId" => json!(RECEIVER),
            "snapshot" => Value::Null,
            _ => json!("shared_loom"),
        };
        assert!(
            parsed(&changed, &op, &context(RECEIVER)).is_err(),
            "{field}"
        );
    }
    let mut changed = value;
    changed["snapshot"]["width"] = 3.into();
    assert!(matches!(
        parsed(&changed, &op, &context(RECEIVER)),
        Err(Error::Image)
    ));
}
#[test]
fn update_accept_cached_read_and_unlink_keep_exact_revision_and_receiver_binding() {
    let first = image(17);
    let second = image(19);
    let envelope = envelope(&first);
    let updated = wire(&envelope, None, 2, second.digest(), Some(RECEIVER));
    let update = Operation::Update {
        envelope: &envelope,
        prior_revision: 1,
        revision: 2,
        image: &second,
    };
    assert!(parsed(&updated, &update, &context(SOURCE)).is_ok());
    let accept = Operation::Accept {
        envelope: &envelope,
        unit: "receive-1",
        revision: 1,
        digest: first.digest(),
        confirmed: true,
    };
    let full = wire(&envelope, Some(&second), 2, second.digest(), Some(RECEIVER));
    assert!(parsed(&full, &accept, &context(RECEIVER)).is_ok());
    let wrong = Operation::Read {
        envelope: &envelope,
        known_revision: 2,
        known_digest: second.digest(),
        receiver_unit: Some("wrong-unit"),
    };
    assert!(parsed(&updated, &wrong, &context(RECEIVER)).is_err());
    let read = Operation::Read {
        envelope: &envelope,
        known_revision: 2,
        known_digest: second.digest(),
        receiver_unit: Some("receive-1"),
    };
    assert!(parsed(&updated, &read, &context(RECEIVER)).is_ok());
    let wrong_digest = Operation::Read {
        envelope: &envelope,
        known_revision: 2,
        known_digest: first.digest(),
        receiver_unit: Some("receive-1"),
    };
    assert!(parsed(&updated, &wrong_digest, &context(RECEIVER)).is_err());
    let read = Operation::Read {
        envelope: &envelope,
        known_revision: 1,
        known_digest: first.digest(),
        receiver_unit: Some("receive-1"),
    };
    assert!(parsed(&updated, &read, &context(RECEIVER)).is_err());
    assert!(parsed(&full, &read, &context(RECEIVER)).is_ok());
    let unlink = Operation::Unlink {
        envelope: &envelope,
    };
    assert!(matches!(
        parsed(&json!({"unlinked":true}), &unlink, &context(RECEIVER)),
        Ok(State::Unlinked)
    ));
    assert!(parsed(&json!({"unlinked":false}), &unlink, &context(RECEIVER)).is_err());
}
#[test]
fn image_decode_cancellation_and_false_dimensions_are_errors_not_empty_images() {
    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        Image::from_png(&png(17), 2, 1, &cancelled),
        Err(Error::Stopped)
    ));
    assert!(matches!(
        Image::from_png(&png(17), 3, 1, &AtomicBool::new(false)),
        Err(Error::Image)
    ));
}
#[test]
#[ignore = "requires an explicitly owned isolated Loom daemon and profile"]
fn actual_loom_signed_out_native_device_lease_is_not_account_authorization() {
    use crate::{device_authorization_owner::Owner, loom_discovery::Config};
    let manifest =
        std::env::var_os("OTTER_PROJECTION_ACCOUNT_MANIFEST").expect("isolated manifest");
    let profile = std::env::var_os("OTTER_PROJECTION_ACCOUNT_PROFILE").expect("isolated profile");
    let config = || {
        Config::from_lookup(|name| {
            (name == std::ffi::OsStr::new("LOOM_MANIFEST_PATH")).then(|| manifest.clone())
        })
    };
    let profile = std::path::PathBuf::from(profile);
    assert!(profile.is_absolute());
    let connected =
        crate::loom_local_connection::connect(config(), &profile, &Cancel::new()).unwrap();
    let mut owner = Owner::new();
    owner
        .install(&connected.origin, connected.session, now().unwrap())
        .unwrap();
    let lease = owner.lease(&connected.origin, now().unwrap()).unwrap();
    assert!(
        matches!(lease.account_projections(&config()), Err(Error::Remote { status: 401, code: Some(code) }) if code == "account_signed_out")
    );
    assert!(!lease.is_revoked()); // Offline accounts must not invalidate signed local Device sessions.
    owner.invalidate(&connected.origin).unwrap();
    assert!(matches!(
        lease.account_projections(&config()),
        Err(Error::Stopped)
    ));
}
