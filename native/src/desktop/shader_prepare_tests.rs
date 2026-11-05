//! 正式端口选择、同份 PNG/straight RGBA 纹理、参数覆盖与失败原子性；不加载 ANGLE。
use super::*;
use crate::{
    desktop::{
        export,
        image::Image,
        loom_image_value::ImageValue,
        loom_input_job::{Job, Source},
        loom_output_batch,
    },
    editor::Editor,
    workspace_id::UnitId,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(crate) fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
pub(crate) fn png(width: u32, height: u32, rgba: &[u8]) -> crate::pixels::Pixels {
    let bgra: crate::pixels::Pixels = rgba
        .chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect::<Vec<_>>()
        .into();
    export::png_straight_bgra(width, height, &bgra, &AtomicBool::new(false)).unwrap()
}
pub(crate) fn encoded(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
pub(crate) fn decode_png(encoded: &str) -> crate::image_decode::shader::Texture {
    let _apartment = Apartment::new().unwrap();
    let bytes = crate::desktop::loom_output_inline::decode_bytes(
        "image/png",
        encoded,
        &AtomicBool::new(false),
    )
    .unwrap();
    crate::image_decode::shader::png(&bytes, 128 * 1024 * 1024, &|| false).unwrap()
}
pub(crate) fn prepared(editor: &Editor, width: u32, height: u32, rgba: &[u8]) -> PreparedBatch {
    let png = png(width, height, rgba);
    let _apartment = Apartment::new().unwrap();
    let image = Image::from_encoded(&png).unwrap();
    let value = ImageValue::from_png(image, png).unwrap();
    let mut results = crate::desktop::loom_result_state::Results::new(id("source"));
    let request = crate::graph_requests::Request {
        id: id("source-request"),
        generation: 1,
    };
    results.begin(request.clone());
    results
        .finish(
            &request,
            loom_output_batch::Batch::local_image(value).unwrap(),
        )
        .unwrap();
    prepare_sources(
        editor,
        vec![
            Source::capture_output(
                "input_path".into(),
                id("source"),
                results.image_source("output").unwrap(),
            )
            .unwrap(),
            Source::capture(
                "reference_path".into(),
                id("reference"),
                1,
                1,
                vec![17, 31, 49, 255].into(),
                &Editor::default(),
            )
            .unwrap(),
        ],
    )
}
pub(crate) fn prepare_sources(editor: &Editor, sources: Vec<Source>) -> PreparedBatch {
    let mut job = Job::default();
    job.start(id("target"), editor, sources).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = job.poll() {
            return result.unwrap();
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
pub(crate) fn batch(value: Value) -> loom_output_batch::Batch {
    loom_output_batch::decode(
        &json!({"output":{"kind":"value","value":value}}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap()
}
pub(crate) fn payload() -> Value {
    json!({"type":"shader","vertex_shader":crate::shader_angle::tests::VERTEX,
        "fragment_shader":crate::shader_angle::tests::FRAGMENT,"uniforms":{"gain":0.25},"textures":{}})
}
pub(crate) fn runtime(backend: Backend, real: bool) -> Runtime {
    let path = if real {
        std::path::PathBuf::from(
            std::env::var_os("OTTER_SHADER_ANGLE_FIXTURE").expect("Set verified ANGLE fixture"),
        )
    } else {
        std::env::temp_dir()
    };
    Runtime::new(
        Some(Arc::new(
            TrustedBundle::new(&path, crate::shader_angle::tests::fixture_identity()).unwrap(),
        )),
        backend,
        "input_path".into(),
        vec![("gain".into(), 1.0)],
    )
    .unwrap()
}
#[test]
fn shader_prepare_preserves_full_straight_rgba_input_and_required_lut_without_pbgra_roundtrip() {
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let editor = Editor::default();
    let matrix = crate::desktop::shader_image::tests::matrix();
    let inputs = prepared(&editor, 256, 256, &matrix.rgba);
    let lut = [
        253, 137, 79, 1, 11, 23, 37, 128, 47, 59, 71, 255, 83, 97, 109, 0,
    ];
    let mut value = payload();
    value["textures"]["lut"] = json!(format!(
        "data:image/png;base64,{}",
        encoded(&png(2, 2, &lut))
    ));
    let shader = runtime(Backend::D3d11, false)
        .prepare(batch(value), &inputs, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(shader.textures.len(), 2);
    assert_eq!(shader.textures[0].name, "input");
    assert_eq!(shader.textures[0].rgba, matrix.rgba);
    // 真实旧 PNG→DOM Image→texImage2D 完整通道/alpha 矩阵 oracle（包括透明隐藏 RGB）。
    assert_eq!(
        crate::desktop::shader_image::tests::hash(&shader.textures[0].rgba),
        "6a26422f3d2bbc33e7ebd143a32f008fafd2c397601c397f1f75bc950e2f56bb"
    );
    assert_eq!(shader.textures[1].name, "lut");
    assert_eq!(&*shader.textures[1].rgba, &lut);
    assert_eq!(shader.uniforms, vec![("gain".into(), 1.0)]);
    assert!(shader
        .textures
        .iter()
        .all(|texture| texture.name != "reference"));
}
#[test]
fn shader_payload_uses_exact_primary_and_first_content_contract() {
    use super::super::shader_payload::Payload;
    let direct = payload();
    let wrapped = json!({"content":[{"text":direct.to_string()},{"text":"ignored"}]});
    assert!(Payload::from_batch(batch(wrapped)).is_ok());
    let invalid = loom_output_batch::decode(&json!({"output_image":{"kind":"value","value":3},"output":{"kind":"value","value":direct}}),None,&AtomicBool::new(false)).unwrap();
    assert!(matches!(
        Payload::from_batch(invalid),
        Err(Error::Unsupported)
    ));
    for field in ["vertex_shader", "fragment_shader"] {
        let mut value = payload();
        value[field] = json!("bad\0source");
        assert!(matches!(
            Payload::from_batch(batch(value)),
            Err(Error::Source)
        ));
    }
    for (field, data) in [
        ("success", json!(false)),
        ("type", json!("other")),
        ("uniforms", json!({"gain":"bad"})),
    ] {
        let mut value = payload();
        value[field] = data;
        assert!(Payload::from_batch(batch(value)).is_err());
    }
    let mut value = payload();
    value["textures"]["input"] = json!("ambiguous");
    assert!(matches!(
        Payload::from_batch(batch(value)),
        Err(Error::Bindings)
    ));
}
#[test]
fn shader_payload_bounds_wrapper_maps_and_source_during_typed_deserialization() {
    use super::super::shader_payload::Payload;
    for (field, count) in [("uniforms", 257), ("textures", 32)] {
        let mut value = payload();
        value[field] = Value::Object(
            (0..count)
                .map(|index| {
                    (
                        format!("binding{index}"),
                        if field == "textures" {
                            json!("")
                        } else {
                            json!(1.0)
                        },
                    )
                })
                .collect(),
        );
        assert!(Payload::from_batch(batch(value.clone())).is_err());
        assert!(
            Payload::from_batch(batch(json!({"content":[{"text":value.to_string()}]}))).is_err()
        );
    }
    let mut value = payload();
    value["vertex_shader"] = json!("x".repeat(crate::shader_worker_wire::MAX_SOURCE + 1));
    assert!(Payload::from_batch(batch(json!({"content":[{"text":value.to_string()}]}))).is_err());
    let duplicate="{\"type\":\"shader\",\"vertex_shader\":\"v\",\"fragment_shader\":\"f\",\"uniforms\":{\"gain\":1,\"gain\":2}}";
    assert!(Payload::from_batch(batch(json!({"content":[{"text":duplicate}]}))).is_err());
}
#[test]
fn shader_prepare_rejects_missing_input_uris_invalid_png_and_cancel_without_partial_invocation() {
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let inputs = prepared(&Editor::default(), 1, 1, &[17, 29, 43, 128]);
    for uri in [
        "https://example.test/lut.png",
        "file:///C:/private.png",
        "data:image/jpeg;base64,AA==",
        "data:image/png;base64,AA==",
    ] {
        let mut value = payload();
        value["textures"]["lut"] = json!(uri);
        assert!(runtime(Backend::D3d11, false)
            .prepare(batch(value), &inputs, &AtomicBool::new(false))
            .is_err());
    }
    assert!(matches!(
        runtime(Backend::D3d11, false).prepare(batch(payload()), &inputs, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    let missing = PreparedBatch::empty_for_test(id("target"), &Editor::default());
    assert!(matches!(
        runtime(Backend::D3d11, false).prepare(batch(payload()), &missing, &AtomicBool::new(false)),
        Err(Error::Bindings)
    ));
    let bytes = png(1, 1, &[1, 2, 3, 4]);
    let _apartment = Apartment::new().unwrap();
    assert!(crate::image_decode::shader::png(&bytes, 3, &|| false).is_err());
    assert!(crate::image_decode::shader::png(&bytes, 4, &|| true).is_err());
    assert!(Runtime::new(
        runtime(Backend::D3d11, false).bundle,
        Backend::D3d11,
        "input_path".into(),
        vec![("bad/path".into(), 1.0)]
    )
    .is_err());
}
