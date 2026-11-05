//! 接受及unchanged/changed都严格核offline route/peer/真实本机设备，PNG仍走原预算和解码。
use super::*;
use crate::projection_delivery::{
    accept::{Read, Received},
    binding::Binding,
    journal::{tests::Directory, Store},
};
#[test]
fn offline_receiver_accept_and_read_validate_route_peer_device_and_unchanged_identity() {
    let item = invitation(ORIGIN, false);
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&item, "device-1").unwrap();
    let image = verified(ORIGIN, "device-1").bind_intent(&ticket).unwrap();
    assert_eq!(&*image.image().pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
    let binding = Binding::new(&image, &ticket).unwrap();
    for (key, value) in [
        ("route", Value::Null),
        ("route", json!("shared_loom")),
        ("peerId", Value::Null),
        ("peerId", json!(format!("loom-{}", "c".repeat(64)))),
        ("offlineTransport", Value::Null),
        ("offlineTransport", json!({"origin":ORIGIN})),
        (
            "delivery",
            json!({"targetDeviceId":"other","status":"accepted"}),
        ),
        (
            "delivery",
            json!({"targetDeviceId":"device-1","status":"awaiting_confirmation"}),
        ),
        ("receiverDeviceId", json!("other")),
    ] {
        let mut wire = response("device-1");
        wire[key] = value;
        assert!(
            Received::parse(
                &serde_json::to_vec(&wire).unwrap(),
                &item,
                ORIGIN,
                "device-1"
            )
            .is_err(),
            "{key}"
        );
        wire["snapshot"] = Value::Null;
        assert!(
            Read::parse(
                &serde_json::to_vec(&wire).unwrap(),
                &ticket,
                &binding,
                ORIGIN,
                "device-1"
            )
            .is_err(),
            "{key}"
        );
    }
    for key in ["route", "peerId", "delivery"] {
        let mut wire = response("device-1");
        wire.as_object_mut().unwrap().remove(key);
        assert!(Received::parse(
            &serde_json::to_vec(&wire).unwrap(),
            &item,
            ORIGIN,
            "device-1"
        )
        .is_err());
    }
    let mut wire = response("device-1");
    wire["snapshot"] = Value::Null;
    let bytes = serde_json::to_vec(&wire).unwrap();
    assert!(matches!(
        Read::parse(&bytes, &ticket, &binding, ORIGIN, "device-1"),
        Ok(Read::Unchanged)
    ));
    assert!(Read::parse(&bytes, &ticket, &binding, SOURCE, "device-1").is_err());
    wire["revision"] = 6.into();
    wire["snapshot"] = response("device-1")["snapshot"].clone();
    assert!(matches!(
        Read::parse(
            &serde_json::to_vec(&wire).unwrap(),
            &ticket,
            &binding,
            ORIGIN,
            "device-1"
        ),
        Ok(Read::Changed(_))
    ));
    wire["snapshot"]["imageBase64"] = "not-png".into();
    assert!(Read::parse(
        &serde_json::to_vec(&wire).unwrap(),
        &ticket,
        &binding,
        ORIGIN,
        "device-1"
    )
    .is_err());
    assert!(matches!(
        Received::parse(
            &serde_json::to_vec(&response("device-1")).unwrap(),
            &item,
            ORIGIN,
            "device-1"
        )
        .unwrap()
        .verify(|| true),
        Err(Error::Stopped)
    ));
}
