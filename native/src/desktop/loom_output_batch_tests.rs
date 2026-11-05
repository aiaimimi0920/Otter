//! 多端口原子交付与主输出选择；不发布窗口或触碰远端租约。
use super::*;
use serde_json::json;
#[test]
fn primary_selection_preserves_main_priority_and_null_values() {
    let mut outputs = json!({"alpha":{"kind":"value","value":null},"image":{"kind":"value","value":3},
        "output":{"kind":"value","value":2},"output_image":{"kind":"value","value":1}});
    for expected in ["output_image", "output", "image", "alpha"] {
        let batch = decode(&outputs, None, &AtomicBool::new(false)).unwrap();
        assert_eq!(batch.primary().name, expected);
        if expected == "alpha" {
            assert!(matches!(batch.primary().value, Output::Value(Value::Null)));
        }
        outputs.as_object_mut().unwrap().remove(expected);
    }
}
#[test]
fn real_image_and_structured_values_are_delivered_together_without_mutating_wire() {
    let cancel = AtomicBool::new(false);
    let input = super::super::loom_inline::prepare(
        1,
        1,
        vec![10, 20, 30, 255].into(),
        &crate::editor::Editor::default(),
        &cancel,
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("png");
    };
    let outputs = json!({"output_image":{"kind":"inline_resource","mime":"image/png","dataBase64":base64},
        "text":{"kind":"value","value":{"items":[1,false,"中文"]}}});
    let before = outputs.clone();
    let batch = decode(&outputs, None, &cancel).unwrap();
    assert_eq!(batch.ports.len(), 2);
    let Output::Image(image) = &batch.primary().value else {
        panic!("image");
    };
    assert_eq!(&*image.pixels, &[10, 20, 30, 255]);
    assert_eq!(outputs, before);
    assert!(
        matches!(&batch.ports[1].value,Output::Value(value) if value==&outputs["text"]["value"])
    );
    let mut invalid = outputs.clone();
    invalid["zzz"] = json!({"kind":"resource"});
    assert!(matches!(
        decode(&invalid, None, &cancel),
        Err(Error::Unsupported)
    ));
    assert_eq!(invalid["output_image"], outputs["output_image"]);
}
#[test]
fn invalid_batches_and_cancellation_never_return_partial_results() {
    let cancel = AtomicBool::new(false);
    for outputs in [
        json!({}),
        json!([]),
        json!({"":{"kind":"value","value":1}}),
        json!({"output":{"kind":"value"}}),
    ] {
        assert!(matches!(
            decode(&outputs, None, &cancel),
            Err(Error::InvalidOutputs)
        ));
    }
    let outputs = json!({"output":{"kind":"value","value":1}});
    let candidates = json!({"items":vec![Value::Null;65]});
    assert!(matches!(
        decode(&outputs, Some(&candidates), &cancel),
        Err(Error::InvalidCandidates)
    ));
    assert!(matches!(
        decode(&outputs, None, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    let map = (0..129)
        .map(|index| (index.to_string(), json!({"kind":"value","value":index})))
        .collect();
    assert!(matches!(
        decode(&Value::Object(map), None, &cancel),
        Err(Error::InvalidOutputs)
    ));
}
