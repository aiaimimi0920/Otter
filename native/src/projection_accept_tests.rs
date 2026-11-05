//! 严格响应/完整 PNG/确定会话身份；图片与协议夹具供原 TLS/runtime 测试共用。
use super::*;
use serde_json::{json, Value};
pub(crate) fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
    }
    bytes
}
pub(crate) fn invitation(origin: &str) -> Invitation {
    super::super::Inbox::parse(&super::super::tests::body(origin), origin)
        .unwrap()
        .into_entries()
        .pop()
        .unwrap()
}
pub(crate) fn accepted(origin: &str) -> Invitation {
    let mut entry = invitation(origin);
    entry.accepted = true;
    entry
}
pub(crate) fn response(origin: &str) -> Value {
    let invitation = invitation(origin);
    let bytes = png();
    json!({"envelope":invitation.envelope(),"revision":5,"digest":format!("{:x}",Sha256::digest(&bytes)),
        "width":2,"height":1,"linked":true,"receiverDeviceId":"device-1",
        "receiverUnitId":invitation.receiver_unit_id(),"snapshot":{"width":2,"height":1,
        "imageBase64":STANDARD.encode(bytes)},"delivery":{"status":"accepted"}})
}
pub(crate) fn received(origin: &str) -> Received {
    Received::parse(
        &serde_json::to_vec(&response(origin)).unwrap(),
        &invitation(origin),
        origin,
        "device-1",
    )
    .unwrap()
}
pub(crate) fn verified(origin: &str) -> Verified {
    std::thread::spawn({
        let origin = origin.to_owned();
        move || {
            unsafe {
                windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
                )
                .ok()
                .unwrap();
            }
            let result = received(&origin).verify(|| false).unwrap();
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
            result
        }
    })
    .join()
    .unwrap()
}
const ORIGIN: &str = "https://loom.example";
#[test]
fn acceptance_keeps_exact_envelope_and_deterministic_id_but_allows_current_resume_snapshot() {
    let mut entry = invitation(ORIGIN);
    entry.accepted = true;
    let mut envelope = serde_json::to_value(entry.envelope()).unwrap();
    envelope["expiresAtMs"] = 1.into();
    entry.envelope = serde_json::from_value(envelope.clone()).unwrap();
    let body: Value = serde_json::from_slice(&entry.accept_body(ORIGIN).unwrap()).unwrap();
    assert_eq!(
        body,
        json!({"envelope":envelope,"expectedRevision":4,
        "expectedDigest":"b".repeat(64),"receiverUnitId":entry.receiver_unit_id(),"confirmed":true})
    );
    let mut reply = response(ORIGIN);
    reply["envelope"] = envelope;
    let accepted = Received::parse(
        &serde_json::to_vec(&reply).unwrap(),
        &entry,
        ORIGIN,
        "device-1",
    )
    .unwrap();
    assert_eq!(accepted.revision, 5);
    assert_ne!(accepted.digest, entry.digest());
    assert_eq!(accepted.png.as_ref(), png());
}
#[test]
fn mismatched_reply_identity_and_revision_never_yield_received_pixels() {
    for (field, value) in [
        ("linked", json!(false)),
        ("receiverDeviceId", json!("other")),
        ("receiverUnitId", json!("other")),
        ("revision", json!(3)),
        ("revision", json!(envelope::MAX_REVISION + 1)),
        ("digest", json!("A".repeat(64))),
        ("snapshot", Value::Null),
    ] {
        let mut reply = response(ORIGIN);
        reply[field] = value;
        assert!(
            Received::parse(
                &serde_json::to_vec(&reply).unwrap(),
                &invitation(ORIGIN),
                ORIGIN,
                "device-1"
            )
            .is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("nonce", json!("3".repeat(32))),
        ("serverOrigin", json!("https://other.example")),
        (
            "signature",
            json!({"algorithm":"ed25519","keyId":"source-1","value":"x".repeat(86)}),
        ),
    ] {
        let mut reply = response(ORIGIN);
        reply["envelope"][field] = value;
        assert!(
            Received::parse(
                &serde_json::to_vec(&reply).unwrap(),
                &invitation(ORIGIN),
                ORIGIN,
                "device-1"
            )
            .is_err(),
            "{field}"
        );
    }
}
#[test]
fn malformed_base64_size_digest_and_duplicate_fields_are_rejected_before_decode() {
    for value in [
        json!(""),
        json!("not-base64"),
        json!("data:image/png;base64,abc"),
        json!(STANDARD.encode(b"not png")),
    ] {
        let mut reply = response(ORIGIN);
        reply["snapshot"]["imageBase64"] = value;
        assert!(Received::parse(
            &serde_json::to_vec(&reply).unwrap(),
            &invitation(ORIGIN),
            ORIGIN,
            "device-1"
        )
        .is_err());
    }
    for (width, height) in [(0, 1), (8193, 1), (8192, 8192), (2, 2)] {
        let mut reply = response(ORIGIN);
        reply["snapshot"]["width"] = width.into();
        reply["snapshot"]["height"] = height.into();
        assert!(Received::parse(
            &serde_json::to_vec(&reply).unwrap(),
            &invitation(ORIGIN),
            ORIGIN,
            "device-1"
        )
        .is_err());
    }
    let mut reply = response(ORIGIN);
    reply["digest"] = "a".repeat(64).into();
    assert!(Received::parse(
        &serde_json::to_vec(&reply).unwrap(),
        &invitation(ORIGIN),
        ORIGIN,
        "device-1"
    )
    .is_err());
    let text = serde_json::to_string(&response(ORIGIN))
        .unwrap()
        .replace("\"revision\":5", "\"revision\":5,\"revision\":6");
    assert!(Received::parse(text.as_bytes(), &invitation(ORIGIN), ORIGIN, "device-1").is_err());
    assert!(matches!(
        Received::parse(
            &vec![b' '; MAX_RESPONSE + 1],
            &invitation(ORIGIN),
            ORIGIN,
            "device-1"
        ),
        Err(Error::Limit)
    ));
}
#[test]
fn actual_decode_rejects_false_dimensions_truncated_png_and_cancel_then_releases_pixels() {
    let valid = verified(ORIGIN);
    assert_eq!((valid.image().width, valid.image().height), (2, 1));
    assert_eq!(&*valid.image().pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
    let weak = valid.image().pixels.downgrade();
    drop(valid);
    assert!(weak.upgrade().is_none());
    std::thread::spawn(|| {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        let mut wrong = received(ORIGIN);
        wrong.width = 1;
        assert!(matches!(wrong.verify(|| false), Err(Error::Snapshot)));
        let mut truncated = received(ORIGIN);
        truncated.png = png()[..33].to_vec().into();
        assert!(matches!(truncated.verify(|| false), Err(Error::Snapshot)));
        assert!(matches!(
            received(ORIGIN).verify(|| true),
            Err(Error::Stopped)
        ));
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    })
    .join()
    .unwrap();
}
#[test]
fn deterministic_session_keeps_projection_and_encoded_budget_without_copying_pixels() {
    let directory = super::super::journal::tests::Directory::new();
    let store = super::super::journal::Store::open(&directory.0).unwrap();
    let intent = store.begin(&invitation(ORIGIN), "device-1").unwrap();
    let image = verified(ORIGIN).bind_intent(&intent).unwrap();
    let pixels = image.image().pixels.clone();
    let (bytes, decoded) = image
        .into_session(
            &intent,
            crate::session_projection_scope::Scope::generate().unwrap(),
            crate::session_placement::Space {
                origin: (-100, 20),
                scale: 1.5,
            },
            crate::session_window::Rect {
                x: 10.0,
                y: 20.0,
                w: 200.0,
                h: 100.0,
            },
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    assert!(decoded.pixels.same_storage(&pixels));
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    let sticker = &value["stickers"][0];
    assert_eq!(sticker["qrProjection"]["localUnitId"], sticker["id"]);
    assert_eq!(
        sticker["qrProjection"]["envelope"],
        response(ORIGIN)["envelope"]
    );
    assert_eq!(sticker["qrProjection"]["revision"], 5);
    assert_eq!(sticker["qrProjection"]["linked"], true);
    assert_eq!(
        value["otterNativeCoordinateSpace"]["origin"],
        json!([-100, 20])
    );
    assert!(crate::session_document::decode(&bytes).is_ok());
}
