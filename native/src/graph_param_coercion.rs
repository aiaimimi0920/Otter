//! 连线参数转换：借用已存在字符串，仅文本化标量/容器时分配；失败不产生候选值。
use crate::graph_image::{validate_outputs, Output};
#[path = "graph_param_number.rs"]
mod number;
#[path = "graph_param_string.rs"]
mod string;
pub(crate) use number::whitespace as initial_whitespace;
pub const MAX_TEXT_BYTES: usize = 65536;
#[derive(Default)]
pub struct Widget<'a> {
    pub name: &'a str,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    Borrowed(Output<'a>),
    Text(String),
}
impl Value<'_> {
    pub fn as_output(&self) -> Output<'_> {
        match self {
            Self::Borrowed(value) => *value,
            Self::Text(text) => Output::Image(text),
        }
    }
}
pub fn coerce<'a>(widget: &Widget<'_>, value: Output<'a>) -> Option<Value<'a>> {
    validate_outputs(&[("value", value)]).ok()?;
    let direct = value.direct();
    let named = |name| widget.name.eq_ignore_ascii_case(name);
    if named("slider") || named("number") {
        let mut result = number::convert(direct)?;
        if let Some(min) = widget.min.filter(|value| value.is_finite()) {
            result = result.max(min);
        }
        if let Some(max) = widget.max.filter(|value| value.is_finite()) {
            result = result.min(max);
        }
        Some(Value::Borrowed(Output::Number(result)))
    } else if named("checkbox") || named("switch") {
        let result = match direct {
            Output::Boolean(value) => value,
            Output::Number(value) => value != 0.0,
            Output::Image(value) => {
                if value.len() > MAX_TEXT_BYTES {
                    return None;
                }
                let value = value.trim_matches(number::whitespace);
                if ["true", "1", "yes", "on"]
                    .iter()
                    .any(|item| value.eq_ignore_ascii_case(item))
                {
                    true
                } else if ["false", "0", "no", "off"]
                    .iter()
                    .any(|item| value.eq_ignore_ascii_case(item))
                {
                    false
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        Some(Value::Borrowed(Output::Boolean(result)))
    } else if named("image_link") || named("file") {
        match direct {
            Output::Image(value) if !value.is_empty() => Some(Value::Borrowed(direct)),
            _ => None,
        }
    } else if named("text") || named("color") {
        if let Output::Image(value) = direct {
            return (value.len() <= MAX_TEXT_BYTES).then_some(Value::Borrowed(direct));
        }
        string::convert(direct).map(Value::Text)
    } else {
        Some(Value::Borrowed(direct))
    }
}
#[cfg(test)]
#[path = "graph_param_coercion_tests.rs"]
mod tests;
