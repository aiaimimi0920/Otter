//! 实际编码结果的协议夹具；不发送到外部服务，预算失败必须保留旧请求和预览。
use super::*;
use crate::{
    graph_execution_config::{Config, Runtime, Trigger},
    graph_image::{DispatchOptions, Graph, Kind, Node, Preparation},
    graph_param_coercion::Widget,
    graph_params::{Param, DISABLED},
};
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn node(unit: &UnitId) -> Node<'_> {
    Node {
        id: unit,
        kind: Kind::Art,
        inputs: &[],
        outputs: &[],
        capability_inputs: None,
        capability_outputs: &[],
        values: &[],
        source: None,
        preview: None,
        manual_image: None,
        input_disabled: false,
        accept_upstream: true,
        locally_edited: false,
        pending: false,
    }
}
fn options() -> DispatchOptions {
    DispatchOptions {
        config: Config::default(),
        runtime: Runtime::default(),
        trigger: Trigger::Manual,
    }
}
#[test]
fn exact_wire_schema_escapes_unicode_controls_and_excludes_secrets() {
    let unit = id("unit");
    let nodes = [node(&unit)];
    let graph = Graph::new(&nodes, &[]).unwrap();
    let declared = [Param {
        id: "secret",
        default: None,
        secret: true,
        widget: Widget::default(),
    }];
    let manual = [
        ("text", Output::Image("中文\"\\\n\u{0001}")),
        ("secret", Output::Image("never-send")),
        ("disabled", Output::Image(DISABLED)),
        ("nonfinite", Output::Number(f64::NAN)),
        ("unset", Output::Other),
    ];
    let Preparation::Ready(prepared) = graph
        .prepare_dispatch(&unit, &declared, &manual, options())
        .unwrap()
    else {
        panic!("prepare");
    };
    let request = Request {
        id: id("request"),
        generation: 1,
    };
    let wire = execute(&unit, "art/demo", &request, &prepared).unwrap();
    assert_eq!(wire, "{\"action\":\"execute_art\",\"payload\":{\"node_id\":\"unit\",\"request_id\":\"request\",\"generation\":1,\"art_id\":\"art/demo\",\"inputs\":{},\"parameters\":{\"text\":\"中文\\\"\\\\\\n\\u0001\",\"nonfinite\":null},\"disabled_parameters\":[\"disabled\"]}}");
    println!("JSON_FIXTURE:{wire}");
    assert_eq!(cancel(&unit, &request).unwrap(), "{\"action\":\"cancel_art\",\"payload\":{\"node_id\":\"unit\",\"request_id\":\"request\",\"generation\":1}}");
}
#[test]
fn staging_encodes_old_cancel_and_only_then_replaces_generation() {
    let unit = id("unit");
    let nodes = [node(&unit)];
    let graph = Graph::new(&nodes, &[]).unwrap();
    let Preparation::Ready(prepared) = graph.prepare_dispatch(&unit, &[], &[], options()).unwrap()
    else {
        panic!("prepare");
    };
    let mut registry = Requests::default();
    let first = stage(&mut registry, &unit, id("first"), "art", &prepared).unwrap();
    assert!(first.cancel.is_none());
    registry
        .mark_preview(&unit, &first.request, "preview".into())
        .unwrap();
    assert!(matches!(
        stage(&mut registry, &unit, id("invalid"), "", &prepared),
        Err(Error::InvalidArtId)
    ));
    assert_eq!(registry.preview(&unit, &first.request), Some("preview"));
    let second = stage(&mut registry, &unit, id("second"), "art", &prepared).unwrap();
    assert_eq!(second.request.generation, 2);
    assert_eq!(
        second.cancel.as_deref(),
        Some(cancel(&unit, &first.request).unwrap().as_str())
    );
    assert!(registry.is_latest(&unit, &second.request));
    assert_eq!(registry.preview_bytes(), 0);
}
#[test]
fn cancel_encoding_failure_preserves_active_request_and_preview() {
    let unit = id("unit");
    let mut registry = Requests::default();
    let old = registry.begin(unit.clone(), id("old")).unwrap();
    registry
        .mark_preview(&unit, &old, "preview".into())
        .unwrap();
    let result = stage_encoded(
        &mut registry,
        &unit,
        id("candidate"),
        |_| Ok("encoded-execute".into()),
        |_| Err(Error::Allocation),
    );
    assert!(matches!(result, Err(Error::Allocation)));
    assert!(registry.is_latest(&unit, &old));
    assert_eq!(registry.preview(&unit, &old), Some("preview"));
    assert_eq!(registry.next_generation(&unit).unwrap(), old.generation + 1);
}
#[test]
fn oversized_encoding_preserves_old_generation_and_preview() {
    let unit = id("unit");
    let nodes = [node(&unit)];
    let graph = Graph::new(&nodes, &[]).unwrap();
    let oversized = "x".repeat(MAX_MESSAGE_BYTES);
    let manual = [("text", Output::Image(&oversized))];
    let Preparation::Ready(prepared) = graph
        .prepare_dispatch(&unit, &[], &manual, options())
        .unwrap()
    else {
        panic!("prepare");
    };
    let mut registry = Requests::default();
    let old = registry.begin(unit.clone(), id("old")).unwrap();
    registry
        .mark_preview(&unit, &old, "preview".into())
        .unwrap();
    assert!(matches!(
        stage(&mut registry, &unit, id("new"), "art", &prepared),
        Err(Error::MessageTooLarge)
    ));
    assert!(registry.is_latest(&unit, &old));
    assert_eq!(registry.preview(&unit, &old), Some("preview"));
    assert_eq!(registry.next_generation(&unit), Ok(2));
}
#[test]
fn json_nested_values_and_size_expansion_are_bounded() {
    let mut json = Json::new();
    json.value(
        Output::Object(&[
            ("skip", Output::Other),
            (
                "array",
                Output::Array(&[
                    Output::Other,
                    Output::Null,
                    Output::Number(-0.0),
                    Output::Boolean(false),
                ]),
            ),
        ]),
        0,
    )
    .unwrap();
    assert_eq!(json.0, "{\"array\":[null,null,0,false]}");
    let mut json = Json::new();
    assert_eq!(
        json.string(&"\u{0001}".repeat(MAX_MESSAGE_BYTES / 6 + 1)),
        Err(Error::MessageTooLarge)
    );
    assert!(json.0.len() <= MAX_MESSAGE_BYTES);
}
