//! 固定wire oracle核协作与压缩PNG的绑定，不由新解码器生成预期。
use super::*;
use serde_json::{json, Value};
const ORIGIN: &str = "https://loom.example";
pub(crate) fn document(wire: &Value, revision: u64) -> Value {
    json!({"schema":"neuro.projection-edit.v1","sessionId":"edit:22222222222222222222222222222222",
        "basis":{"digest":wire["digest"],"width":wire["width"],"height":wire["height"]},
        "revision":revision,"modeRevision":1,"checkpointRevision":1,"receiptCount":revision-1,"mode":"one_way",
        "objects":{"remote":{"revision":revision,"value":{"id":"remote","type":"rect","zIndex":0,
            "style":{"color":"#0000ff","fill":"#0000ff","width":0},"x":0,"y":0,"w":1,"h":1}}}})
}
fn parse(wire: &Value) -> Result<Received, Error> {
    Received::parse(
        &serde_json::to_vec(wire).unwrap(),
        &tests::invitation(ORIGIN),
        ORIGIN,
        "device-1",
    )
}
#[test]
fn editing_accept_keeps_complete_document_and_independent_png() {
    let mut wire = tests::response(ORIGIN);
    wire["editing"] = document(&wire, 1);
    let received = parse(&wire).unwrap();
    assert_eq!(received.png.as_ref(), tests::png());
    assert_eq!(
        serde_json::to_value(received.editing.unwrap()).unwrap(),
        wire["editing"]
    );
}
#[test]
fn editing_accept_rejects_mismatched_basis_invalid_objects_and_document_budget() {
    let mut good = tests::response(ORIGIN);
    good["editing"] = document(&good, 1);
    let mut bad = Vec::new();
    for (key, value) in [
        ("digest", json!("a".repeat(64))),
        ("width", json!(3)),
        ("height", json!(2)),
    ] {
        let mut wire = good.clone();
        wire["editing"]["basis"][key] = value;
        bad.push(wire);
    }
    let mut wire = good.clone();
    wire["editing"]["objects"]["remote"]["value"]["type"] = json!("blur");
    bad.push(wire);
    let mut wire = good.clone();
    wire["editing"] = json!({"padding":"x".repeat(crate::projection_edit::MAX_BYTES)});
    bad.push(wire);
    let mut wire = good;
    wire["editing"]["sessionId"] = json!("edit:wrong");
    bad.push(wire);
    for wire in bad {
        assert!(parse(&wire).is_err());
    }
}
#[test]
fn editing_session_has_scope_vectors_and_digest_bound_snapshot_without_baking() {
    std::thread::spawn(|| {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        let directory = super::super::journal::tests::Directory::new();
        let store = super::super::journal::Store::open(&directory.0).unwrap();
        let ticket = store.begin(&tests::invitation(ORIGIN), "device-1").unwrap();
        let mut wire = tests::response(ORIGIN);
        wire["editing"] = document(&wire, 1);
        let value = parse(&wire)
            .unwrap()
            .verify(|| false)
            .unwrap()
            .bind_intent(&ticket)
            .unwrap();
        let pixels = value.image().pixels.clone();
        let scope = crate::session_projection_scope::Scope::generate().unwrap();
        let (bytes, image) = value
            .into_session(
                &ticket,
                scope,
                crate::session_placement::Space {
                    origin: (0, 0),
                    scale: 1.0,
                },
                crate::session_window::Rect {
                    x: 20.0,
                    y: 30.0,
                    w: 160.0,
                    h: 80.0,
                },
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
        assert!(image.pixels.same_storage(&pixels));
        assert_eq!(&*pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let shape = &value["stickers"][0]["annotationState"]["elements"][0];
        assert_eq!(
            (
                shape["x"].as_f64(),
                shape["y"].as_f64(),
                shape["w"].as_f64(),
                shape["h"].as_f64()
            ),
            (Some(0.0), Some(0.0), Some(80.0), Some(80.0))
        );
        let decoded = crate::session_document::decode(&bytes).unwrap();
        let snapshot =
            crate::projection_edit::workspace::Snapshot::restore(&decoded.windows[0], Some(scope))
                .unwrap()
                .unwrap();
        assert_eq!(
            serde_json::to_value(snapshot.document()).unwrap(),
            wire["editing"]
        );
        assert_eq!(
            snapshot.role(),
            crate::projection_edit::workspace::Role::Receiver
        );
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    })
    .join()
    .unwrap();
}
