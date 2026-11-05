//! 真实 WIC PNG 编解码回环及正式输出校验，不发布图片或访问用户资源。
use super::*;
use crate::{editor::Editor, loom_art_wire::Input};
use serde_json::json;

#[test]
fn actual_png_output_roundtrip_uses_decoded_dimensions_and_pixel_budget_owner() {
    let cancel = AtomicBool::new(false);
    let pixels: crate::pixels::Pixels = vec![10, 20, 30, 255, 40, 50, 60, 255].into();
    let input =
        super::super::loom_inline::prepare(2, 1, pixels.clone(), &Editor::default(), &cancel)
            .unwrap();
    let Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("inline");
    };
    let value = json!({"kind":"inline_resource","mime":"image/png","dataBase64":base64,"width":999999999,"height":0});
    let wire = json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":"loom.hook.art.result",
        "params":{"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":1,
        "resultRevision":1,"outputs":{"output":value}}});
    let request = crate::graph_requests::Request {
        id: crate::workspace_id::UnitId::parse("request").unwrap(),
        generation: 1,
    };
    let event = crate::loom_art_response::parse(
        &serde_json::to_vec(&wire).unwrap(),
        &crate::workspace_id::UnitId::parse("unit").unwrap(),
        &request,
    )
    .unwrap();
    let crate::loom_art_response::Event::Result { outputs, .. } = event else {
        panic!("result");
    };
    let image = decode(&outputs["output"], &cancel).unwrap();
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.pixels, pixels);
    let weak = image.pixels.downgrade();
    let clone = image.clone();
    drop(image);
    assert!(weak.upgrade().is_some());
    drop(clone);
    assert!(weak.upgrade().is_none());
}

#[test]
fn invalid_kind_mime_data_and_nonimage_payload_are_rejected() {
    let cancel = AtomicBool::new(false);
    for value in [
        json!({"kind":"resource","mime":"image/png","dataBase64":"YWJj"}),
        json!({"kind":"inline_resource","mime":"image/svg+xml","dataBase64":"YWJj"}),
        json!({"kind":"inline_resource","mime":"image/png","dataBase64":""}),
        json!({"kind":"inline_resource","mime":"image/png","dataBase64":"data:image/png;base64,YWJj"}),
        json!({"kind":"inline_resource","mime":"image/png","dataBase64":"%%%"}),
        json!({"kind":"inline_resource","mime":"image/png","dataBase64":"YWJj"}),
    ] {
        assert!(decode(&value, &cancel).is_err());
    }
}

#[test]
fn cancellation_precedes_allocation_and_invalid_payload_handling() {
    let error = match decode(&Value::Null, &AtomicBool::new(true)) {
        Ok(_) => panic!("cancel"),
        Err(error) => error,
    };
    assert_eq!(error.code(), E_ABORT);
}
