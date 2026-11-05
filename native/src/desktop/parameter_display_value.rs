//! 对照 UnitParamsPanel 的显示回退和控件值转换；只生成初值，不回写公共参数。
use crate::loom_capability::Parameter;
use serde_json::Value;
pub(super) fn effective<'a>(
    parameter: &Parameter<'a>,
    current: Option<&'a Value>,
) -> Option<&'a Value> {
    current
        .filter(|value| !value.is_null() && value.as_str() != Some(crate::graph_params::DISABLED))
        .or(parameter.default)
        .filter(|value| !value.is_null())
}
pub(super) fn boolean(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}
pub(super) fn text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Number(value)) => value
            .as_f64()
            .map_or_else(|| value.to_string(), |value| value.to_string()),
        Some(Value::Object(_)) => "[object Object]".into(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| text(Some(value)))
            .collect::<Vec<_>>()
            .join(","),
    }
}
pub(super) fn number(value: Option<&Value>) -> Option<f64> {
    match value {
        None | Some(Value::Null) => Some(0.0),
        Some(Value::Bool(value)) => Some(if *value { 1.0 } else { 0.0 }),
        Some(Value::Number(value)) => value.as_f64(),
        Some(Value::String(value)) => super::parameter_number::parse(value),
        Some(Value::Array(_)) => super::parameter_number::parse(&text(value)),
        Some(Value::Object(_)) => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_javascript_control_coercions_do_not_mutate_source() {
        let raw = serde_json::json!({"id":"art","parameters":[{"id":"value","default":7}]});
        let capability = crate::loom_capability::Capability::new(&raw).unwrap();
        let parameter = capability.parameters().next().unwrap();
        for current in [
            None,
            Some(Value::Null),
            Some(Value::String(crate::graph_params::DISABLED.into())),
        ] {
            assert_eq!(
                effective(&parameter, current.as_ref()),
                Some(&Value::from(7))
            );
        }
        assert!(boolean(Some(&Value::String("false".into()))));
        assert!(boolean(Some(&serde_json::json!([]))));
        assert!(!boolean(Some(&Value::from(0))));
        assert_eq!(number(Some(&serde_json::json!(["3"]))), Some(3.0));
        assert_eq!(number(Some(&serde_json::json!([]))), Some(0.0));
        assert_eq!(number(Some(&serde_json::json!({}))), None);
        assert_eq!(
            text(Some(&serde_json::json!([1, null, [2, 3], {}]))),
            "1,,2,3,[object Object]"
        );
    }
}
