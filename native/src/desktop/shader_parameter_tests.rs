//! 参数类型、冻结、公共过滤及覆盖的无 GPU 门；WIC/PreparedBatch 复用原生产 owner。
use super::*;
use crate::{
    desktop::shader_prepare::{self, tests as staging},
    editor::Editor,
    graph_param_coercion::Widget,
    graph_params::Param,
    shader_angle::Backend,
};
use std::sync::atomic::AtomicBool;

#[test]
fn shader_parameters_freeze_resolved_numbers_booleans_and_png_without_secret_or_reference() {
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let uri = format!(
        "data:image/png;base64,{}",
        staging::encoded(&staging::png(1, 1, &[9, 17, 33, 255]))
    );
    let declared = [Param {
        id: "secret",
        secret: true,
        default: Some(Output::Number(99.0)),
        widget: Widget {
            name: "number",
            min: None,
            max: None,
        },
    }];
    let values = [
        ("secret", Output::Number(77.0)),
        ("gain", Output::Number(0.5)),
        ("enabled", Output::Boolean(true)),
        ("overlay", Output::Image(&uri)),
        ("reference", Output::Image("data:ignored")),
        ("__ui", Output::Number(99.0)),
        ("mode", Output::Image("cpu")),
        ("empty", Output::Image("")),
        ("disabled", Output::Image(crate::graph_params::DISABLED)),
    ];
    let parameters = Params::new(&declared, &values).unwrap();
    let bindings = Bindings::capture(&parameters, Vec::new()).unwrap();
    assert_eq!(
        bindings.uniforms,
        vec![("gain".into(), 0.5), ("enabled".into(), 1.0)]
    );
    assert_eq!(bindings.textures.len(), 1);
    assert_eq!(bindings.textures.get("overlay"), Some(&uri));
    let raw = serde_json::json!({"id":"shader","metadata":{"capabilities":{"shaderInput":"source"}},"inputs":[{"name":"source","type":"image"}]});
    let installation = shader_prepare::Installation {
        bundle: staging::runtime(Backend::D3d11, false).bundle,
        backend: Backend::D3d11,
    };
    // 指定端口缺失时只回退到原先的标准输入名，不误取 reference。
    assert!(installation
        .capture(
            &crate::loom_capability::Capability::new(&raw).unwrap(),
            &["reference"],
            &parameters,
            &[]
        )
        .is_err());
    let raw = serde_json::json!({"id":"shader","inputs":[{"name":"input_path","type":"image"}]});
    let runtime = installation
        .capture(
            &crate::loom_capability::Capability::new(&raw).unwrap(),
            &["input_path", "reference_path"],
            &parameters,
            &[],
        )
        .unwrap();
    drop(parameters);
    drop(uri);
    let inputs = staging::prepared(&Editor::default(), 1, 1, &[64, 32, 16, 255]);
    let mut payload = staging::payload();
    payload["textures"]["overlay"] = serde_json::json!("data:image/png;base64,invalid");
    let prepared = runtime
        .prepare(staging::batch(payload), &inputs, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        &*prepared
            .textures
            .iter()
            .find(|texture| texture.name == "overlay")
            .unwrap()
            .rgba,
        &[9, 17, 33, 255]
    );
    assert_eq!(
        prepared.uniforms,
        vec![("enabled".into(), 1.0), ("gain".into(), 0.5)]
    );
}
#[test]
fn shader_parameters_reject_nonfinite_gpu_values_ambiguous_primary_and_unsupported_image_data() {
    for value in [
        Output::Number(f64::MAX),
        Output::Number(f64::NAN),
        Output::Number(f64::INFINITY),
        Output::Image("data:image/jpeg;base64,AA=="),
    ] {
        let values = [("gain", value)];
        let result = Params::new(&[], &values).and_then(|parameters| {
            Bindings::capture(&parameters, Vec::new())
                .map_err(|_| crate::graph_image::Error::InvalidOutput)
        });
        assert!(result.is_err());
    }
    for (name, value) in [
        ("input", Output::Image("data:image/png;base64,AA==")),
        ("bad/path", Output::Number(1.0)),
    ] {
        let values = [(name, value)];
        assert!(Bindings::capture(&Params::new(&[], &values).unwrap(), Vec::new()).is_err());
    }
}

#[test]
fn shader_prepared_parameters_require_public_image_declarations_and_exclude_reserved_or_disabled() {
    let raw = serde_json::json!({"id":"shader","parameters":[
        {"id":"overlay","widget":"image_link"},{"id":"overlay","widget":"file"},
        {"id":"lut","data_type":"image"},{"id":"secret","secret":true,"widget":"file"},
        {"id":"reference","widget":"file"},{"id":"input","widget":"file"},
        {"id":"input_path","widget":"file"},{"id":"__ui","widget":"file"},
        {"id":"disabled","widget":"file"},{"id":"gain","widget":"number"}]});
    let ports = [
        "overlay",
        "lut",
        "secret",
        "reference",
        "input",
        "input_path",
        "__ui",
        "disabled",
        "gain",
        "undeclared",
    ];
    let values: Vec<_> = ports
        .iter()
        .map(|name| {
            (
                *name,
                if *name == "disabled" {
                    Output::Image(crate::graph_params::DISABLED)
                } else {
                    Output::Null
                },
            )
        })
        .collect();
    let parameters = Params::new(&[], &values).unwrap();
    assert_eq!(
        prepared_parameters(
            &crate::loom_capability::Capability::new(&raw).unwrap(),
            &parameters,
            "input_path",
            &ports,
            &[]
        )
        .unwrap(),
        ["overlay", "lut"]
    );
    for names in [
        vec!["bad/path".into()],
        (0..32).map(|n| format!("texture{n}")).collect(),
    ] {
        let raw = serde_json::json!({"id":"shader","parameters":names.iter().map(|name|
            serde_json::json!({"id":name,"widget":"image_link"})).collect::<Vec<_>>()});
        let ports: Vec<_> = names.iter().map(String::as_str).collect();
        let values: Vec<_> = ports.iter().map(|name| (*name, Output::Null)).collect();
        assert!(prepared_parameters(
            &crate::loom_capability::Capability::new(&raw).unwrap(),
            &Params::new(&[], &values).unwrap(),
            "input_path",
            &ports,
            &[]
        )
        .is_err());
    }
}

#[test]
fn shader_prepared_parameter_uses_exact_prepared_png_over_manual_and_response_without_fallback() {
    use crate::desktop::loom_input_job::Source;
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let editor = Editor::default();
    let rgba = [253, 137, 79, 1, 83, 97, 109, 0];
    let input = format!(
        "data:image/png;base64,{}",
        staging::encoded(&staging::png(1, 1, &[32, 64, 96, 255]))
    );
    let overlay = format!(
        "data:image/png;base64,{}",
        staging::encoded(&staging::png(2, 1, &rgba))
    );
    let inputs = staging::prepare_sources(
        &editor,
        [
            ("input_path", input.as_str()),
            ("overlay", overlay.as_str()),
        ]
        .into_iter()
        .map(|(name, uri)| {
            Source::capture_inline(name.into(), staging::id("target"), uri, &editor).unwrap()
        })
        .collect(),
    );
    let raw = serde_json::json!({"id":"shader","inputs":[{"name":"input_path","type":"image"}],
        "parameters":[{"id":"overlay","widget":"image_link"}]});
    let values = [(
        "overlay",
        Output::Image("data:image/jpeg;base64,invalid-manual"),
    )];
    let parameters = Params::new(&[], &values).unwrap();
    let installation = shader_prepare::Installation {
        bundle: staging::runtime(Backend::D3d11, false).bundle,
        backend: Backend::D3d11,
    };
    let runtime = || {
        installation
            .capture(
                &crate::loom_capability::Capability::new(&raw).unwrap(),
                &["input_path", "overlay"],
                &parameters,
                &[],
            )
            .unwrap()
    };
    let mut payload = staging::payload();
    payload["textures"]["overlay"] = serde_json::json!("https://example.test/not-fetched.png");
    let invocation = runtime()
        .prepare(
            staging::batch(payload.clone()),
            &inputs,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(invocation.textures.len(), 2);
    let texture = invocation
        .textures
        .iter()
        .find(|t| t.name == "overlay")
        .unwrap();
    assert_eq!((texture.width, texture.height), (2, 1));
    assert_eq!(
        &*texture.rgba, &rgba,
        "no PBGRA quantization or loss of transparent hidden RGB"
    );
    let missing = staging::prepared(&editor, 1, 1, &[32, 64, 96, 255]);
    assert!(matches!(
        runtime().prepare(
            staging::batch(payload.clone()),
            &missing,
            &AtomicBool::new(false)
        ),
        Err(Error::Bindings)
    ));
    assert!(matches!(
        runtime().prepare(staging::batch(payload), &inputs, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    let mut too_many = staging::payload();
    too_many["textures"] = serde_json::Value::Object(
        (0..31)
            .map(|n| (format!("texture{n}"), serde_json::json!("")))
            .collect(),
    );
    assert!(matches!(
        runtime().prepare(staging::batch(too_many), &inputs, &AtomicBool::new(false)),
        Err(Error::Bindings)
    ));
}
