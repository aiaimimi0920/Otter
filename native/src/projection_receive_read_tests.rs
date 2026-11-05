//! 真实 nullable snapshot 合同、关联与单调性反例，不把 HTTP 200 当作新帧。
use super::*;
use crate::projection_delivery::{
    accept::tests as png,
    journal::{tests::Directory, Store},
};
use serde_json::json;
const ORIGIN: &str = "https://loom.example";
fn check(mut change: impl FnMut(&mut serde_json::Value)) -> Result<Read, Error> {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&png::invitation(ORIGIN), "device-1").unwrap();
    let value = png::verified(ORIGIN).bind_intent(&ticket).unwrap();
    let expected = Binding::new(&value, &ticket).unwrap();
    let mut wire = png::response(ORIGIN);
    wire["snapshot"] = serde_json::Value::Null;
    change(&mut wire);
    Read::parse(
        &serde_json::to_vec(&wire).unwrap(),
        &ticket,
        &expected,
        ORIGIN,
        "device-1",
    )
}
#[test]
fn projection_sync_read_null_and_omitted_unchanged_have_no_pixels() {
    assert!(matches!(check(|_| {}), Ok(Read::Unchanged)));
    assert!(matches!(
        check(|v| {
            v.as_object_mut().unwrap().remove("snapshot");
        }),
        Ok(Read::Unchanged)
    ));
}
#[test]
fn projection_sync_read_changed_requires_complete_snapshot_and_monotonic_identity() {
    for (key, value) in [
        ("revision", json!(4)),
        ("revision", json!(6)),
        ("digest", json!("a".repeat(64))),
        ("linked", json!(false)),
        ("receiverDeviceId", json!("other")),
        ("receiverUnitId", json!("other")),
        ("width", json!(0)),
    ] {
        assert!(
            check(|v| {
                v[key] = value.clone();
            })
            .is_err(),
            "{key}"
        );
    }
    assert!(check(|v| {
        v["envelope"]["nonce"] = json!("3".repeat(32));
    })
    .is_err());
    assert!(matches!(
        check(|v| {
            v["revision"] = json!(6);
            v["snapshot"] = png::response(ORIGIN)["snapshot"].clone();
        }),
        Ok(Read::Changed(_))
    ));
}
#[test]
fn projection_sync_read_same_revision_snapshot_is_still_verified_and_digest_stable() {
    assert!(matches!(
        check(|v| {
            v["snapshot"] = png::response(ORIGIN)["snapshot"].clone();
        }),
        Ok(Read::Changed(_))
    ));
    assert!(check(|v| {
        v["snapshot"] = png::response(ORIGIN)["snapshot"].clone();
        v["snapshot"]["imageBase64"] = json!("not-png");
    })
    .is_err());
}

#[test]
fn projection_sync_read_same_png_revision_retains_editing_and_rejects_bad_metadata() {
    let doc = super::super::edit_tests::document(&png::response(ORIGIN), 3);
    let value = check(|wire| wire["editing"] = doc.clone()).unwrap();
    let Read::Editing(document) = value else {
        panic!("editing metadata lost")
    };
    assert_eq!(document.revision(), 3);
    for value in [
        json!({}),
        json!({"schema":"neuro.projection-edit.v1"}),
        json!("wrong"),
    ] {
        assert!(check(|wire| wire["editing"] = value.clone()).is_err());
    }
    assert!(check(|wire| {
        wire["editing"] = doc.clone();
        wire["editing"]["basis"]["digest"] = json!("a".repeat(64));
    })
    .is_err());
}
