//! 使用真实 WIC 编码验证混合端口保存；不创建窗口或访问外部资源。
use super::tests::{data, doc};
use super::*;
use crate::desktop::image::Image;

fn image() -> Image {
    Image {
        width: 1,
        height: 1,
        pixels: vec![10, 20, 30, 255].into(),
    }
}

#[test]
fn mixed_outputs_encode_images_and_keep_values_in_the_same_session() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut document = doc("node:test");
    document.outputs =
        Some(serde_json::from_value(serde_json::json!({"text":"hello", "zero":0})).unwrap());
    document.output_images = vec![
        ("output".into(), image().into()),
        ("output_image".into(), image().into()),
    ];
    let saved = data(vec![document])
        .encode(&AtomicBool::new(false))
        .unwrap();
    let json: Value = serde_json::from_slice(&saved.bytes).unwrap();
    let outputs = &json["stickers"][0]["outputs"];
    assert_eq!(outputs["text"], "hello");
    assert_eq!(outputs["zero"], 0);
    let url = outputs["output"].as_str().unwrap();
    assert!(url.starts_with("data:image/png;base64,"));
    let decoded = crate::desktop::session_image::inline(url, &AtomicBool::new(false)).unwrap();
    assert_eq!((decoded.width, decoded.height), (1, 1));
    assert_eq!(decoded.pixels.as_ref(), image().pixels.as_ref());
    assert_eq!(outputs["output"], outputs["output_image"]);
    assert_eq!(outputs.as_object().unwrap().len(), 4);
}

#[test]
fn duplicate_image_port_and_cancel_do_not_return_partial_candidate() {
    for cancelled in [false, true] {
        let mut document = doc("node:test");
        document.outputs =
            Some(serde_json::from_value(serde_json::json!({"output":"conflict"})).unwrap());
        document.output_images = vec![("output".into(), image().into())];
        assert!(data(vec![document])
            .encode(&AtomicBool::new(cancelled))
            .is_err());
    }
}

#[test]
fn shader_png_formal_outputs_survive_actual_session_save_restore_without_reencoding() {
    use crate::desktop::{
        loom_result_state::Results,
        shader_image::{self, tests as oracle},
    };
    let cancel = AtomicBool::new(false);
    let output = shader_image::prepare(oracle::matrix(), &cancel).unwrap();
    let png = output.png.clone();
    let request = crate::graph_requests::Request {
        id: crate::workspace_id::UnitId::parse("shader-request").unwrap(),
        generation: 1,
    };
    let mut results = Results::new(crate::workspace_id::UnitId::parse("node:test").unwrap());
    results.begin(request.clone());
    results
        .finish(&request, output.into_batch().unwrap())
        .unwrap();
    let mut document = doc("node:test");
    document.art = true;
    document.output_images = results.save_output_images();
    document.outputs = results.save_value_outputs();
    let mut snapshot = data(vec![document]);
    snapshot.original = crate::session_source::Source::from(
        br#"{"stickers":[{"id":"node:test","type":"art","x":0,"y":0,"w":256,"h":256}],"links":[]}"#
            .to_vec(),
    );
    let saved = snapshot.encode(&cancel).unwrap();
    let json: Value = serde_json::from_slice(&saved.bytes).unwrap();
    let expected = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    assert_eq!(json["stickers"][0]["outputs"]["output"], expected.as_str());
    assert_eq!(
        json["stickers"][0]["outputs"]["output_image"],
        expected.as_str()
    );
    let candidate =
        crate::desktop::session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
    let restored = candidate.prepare_results(&cancel).unwrap();
    let source = restored[0].image_source("output_image").unwrap();
    assert_eq!(source.image.encoded_png().unwrap().as_ref(), png.as_ref());
    assert_eq!(
        oracle::hash(&oracle::decode_rgba(
            source.image.encoded_png().unwrap(),
            256,
            256
        )),
        oracle::STRAIGHT_SHA256
    );
    let inline = crate::desktop::loom_inline::prepare_value(source.image, &cancel).unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = inline.descriptor() else {
        panic!("PNG required")
    };
    assert_eq!(format!("data:image/png;base64,{base64}"), expected.as_str());
}
