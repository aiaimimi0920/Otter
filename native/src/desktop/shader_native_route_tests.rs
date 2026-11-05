//! 原生产品取码/输入准备/隔离渲染/正式PNG的接缝回归；不把私有worker入口当产品路由。
use super::*;
use serde_json::json;
use std::sync::atomic::AtomicBool;
const SOURCE: &str =
    "float4 otter_main(float2 uv) { return u_input.Sample(otter_sampler, uv) * u_gain; }";
fn payload() -> serde_json::Value {
    json!({"type":"shader","abi":crate::shader_hlsl::ABI,"source":SOURCE,"uniforms":{"gain":0.25},"textures":{}})
}
#[test]
fn native_contract_is_explicit_and_never_reinterprets_legacy_glsl() {
    use shader_prepare::tests::batch;
    assert!(shader_payload::Payload::from_batch(batch(payload()))
        .unwrap()
        .is_native());
    for value in [
        json!({"type":"shader","abi":"otter.hlsl.v2","source":SOURCE}),
        json!({"type":"shader","source":SOURCE}),
        json!({"type":"shader","abi":crate::shader_hlsl::ABI,"source":SOURCE,"vertex_shader":"legacy"}),
        json!({"type":"shader","abi":crate::shader_hlsl::ABI,"source":""}),
    ] {
        assert!(shader_payload::Payload::from_batch(batch(value)).is_err());
    }
}
#[test]
fn native_preparation_rejects_legacy_recipe_before_any_child() {
    let _serial = loom_input_job::TEST_SERIAL.lock().unwrap();
    let inputs =
        shader_prepare::tests::prepared(&crate::editor::Editor::default(), 1, 1, &[1, 2, 3, 255]);
    let runtime = shader_prepare::Runtime::new(
        None,
        crate::shader_angle::Backend::D3d11,
        "input_path".into(),
        vec![],
    )
    .unwrap();
    assert!(matches!(
        runtime.acquire(
            shader_prepare::tests::batch(shader_prepare::tests::payload()),
            &inputs,
            &AtomicBool::new(false)
        ),
        Err(shader_payload::Error::Unsupported)
    ));
}
#[test]
#[ignore = "explicit native GPU product route gate"]
fn native_recipe_reaches_isolated_auto_child_and_preserves_formal_straight_png() {
    render_recipe(payload());
}
#[test]
#[ignore = "requires explicit same-run Loom HLSL sample output and native GPU"]
fn loom_native_sample_source_reaches_formal_png_without_angle() {
    use std::io::Read;
    let path = std::env::var_os("OTTER_HLSL_SAMPLE_OUTPUT").expect("same-run sample output");
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .unwrap()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    render_recipe(serde_json::from_slice(&bytes).unwrap());
}
fn render_recipe(value: serde_json::Value) {
    let _serial = loom_input_job::TEST_SERIAL.lock().unwrap();
    let _apartment = Apartment::new().unwrap();
    let rgba = [255, 128, 64, 128, 16, 255, 77, 0, 1, 2, 3, 255];
    let inputs = shader_prepare::tests::prepared(&crate::editor::Editor::default(), 3, 1, &rgba);
    let runtime = shader_prepare::Runtime::new(
        None,
        crate::shader_angle::Backend::D3d11,
        "input_path".into(),
        vec![("gain".into(), 1.0)],
    )
    .unwrap();
    let (invocation, recipe) = runtime
        .acquire(
            shader_prepare::tests::batch(value),
            &inputs,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(recipe.is_some());
    assert!(matches!(
        &invocation,
        shader_invocation::Invocation::Native(_)
    ));
    let frame = invocation.render(&|| false).unwrap();
    drop(invocation);
    let output = shader_image::prepare(frame, &AtomicBool::new(false)).unwrap();
    let decoded = crate::image_decode::shader::png(&output.png, 1024, &|| false).unwrap();
    assert_eq!(&*decoded.rgba, &rgba);
    assert_eq!(
        &*output.image.pixels,
        &[32, 64, 128, 128, 0, 0, 0, 0, 3, 2, 1, 255]
    );
    assert!(output.into_batch().is_ok());
}
