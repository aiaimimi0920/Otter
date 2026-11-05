//! 实际解码 JSON 到图转换器的借用适配，保持一层解包和结构预算。
use super::*;
use crate::graph_param_coercion::{coerce, Widget};
use serde_json::json;
#[test]
fn decoded_wrappers_preserve_falsy_priority_and_unwrap_only_once() {
    for value in [json!(false), json!(0), json!("")] {
        let wrapped = json!({"value": value, "src":"wrong"});
        assert_eq!(
            Output::from_json(&wrapped).direct(),
            Output::from_json(&value)
        );
    }
    let nested = json!({"value":{"data":"nested"},"src":"wrong"});
    let Output::Json(inner) = Output::from_json(&nested).direct() else {
        panic!("one layer only")
    };
    assert!(std::ptr::eq(inner, &nested["value"]));
    let wrapper = json!({"value":null,"data":false,"src":"wrong"});
    assert_eq!(Output::from_json(&wrapper).direct(), Output::Boolean(false));
}
#[test]
fn decoded_containers_reuse_widget_conversion_without_tree_copy() {
    let value = json!([1, null, [false, "文本"], {"key":"value"}]);
    let converted = coerce(
        &Widget {
            name: "text",
            ..Default::default()
        },
        Output::from_json(&value),
    )
    .unwrap();
    assert_eq!(
        converted.as_output(),
        Output::Image("1,,false,文本,[object Object]")
    );
    let number = json!({"value":["12.5"]});
    let converted = coerce(
        &Widget {
            name: "number",
            min: None,
            max: Some(10.0),
        },
        Output::from_json(&number),
    )
    .unwrap();
    assert_eq!(converted.as_output(), Output::Number(10.0));
    let converted = coerce(&Widget::default(), Output::from_json(&value)).unwrap();
    let Output::Json(borrowed) = converted.as_output() else {
        panic!("container preserved")
    };
    assert!(std::ptr::eq(borrowed, &value));
}
#[test]
fn decoded_structure_obeys_same_limits_as_graph_values() {
    let mut value = json!(0);
    for _ in 0..34 {
        value = json!([value]);
    }
    assert_eq!(
        validate_outputs(&[("out", Output::from_json(&value))]),
        Err(Error::Capacity)
    );
    let value = json!(vec![0; 4096]);
    assert_eq!(
        validate_outputs(&[("out", Output::from_json(&value))]),
        Err(Error::Capacity)
    );
    let value = json!({"x".repeat(257): 0});
    assert_eq!(
        validate_outputs(&[("out", Output::from_json(&value))]),
        Err(Error::InvalidOutput)
    );
}
#[test]
fn decoded_formal_output_flows_through_graph_parameter_resolution() {
    use super::tests::{id, link, sticker};
    let ids = [id("source"), id("target")];
    let value = json!({"value":"12.5"});
    let values = [("output_image", Output::from_json(&value))];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    nodes[0].values = &values;
    nodes[1].kind = Kind::Art;
    let edges = [link("edge", &ids[0], &ids[1], "amount")];
    let params = [crate::graph_params::Param {
        id: "amount",
        default: None,
        secret: false,
        widget: Widget {
            name: "number",
            min: None,
            max: Some(10.0),
        },
    }];
    let graph = Graph::new(&nodes, &edges).unwrap();
    let resolved = graph.effective_params(&ids[1], &params, &[]).unwrap();
    assert_eq!(
        resolved.entries().next(),
        Some(("amount", Some(Output::Number(10.0))))
    );
}
