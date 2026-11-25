//! 节点初始默认值，对齐 artNodeFactory；只转换字符串，不把控件范围当初始化约束。
use super::{Capability, Parameter};
use crate::graph_image::Output;
use crate::graph_param_coercion::{coerce, Widget};
use serde_json::Value;
use std::borrow::Cow;

#[derive(Debug, PartialEq)]
pub enum Error {
    Capacity,
}
pub type InitialParameters<'a> = Vec<(&'a str, Option<Cow<'a, Value>>)>;

fn schema_type(schema: &Value, depth: usize, budget: &mut usize) -> Result<Option<String>, Error> {
    if depth > 64 || *budget == 0 {
        return Err(Error::Capacity);
    }
    *budget -= 1;
    if let Some(name) = schema.get("type").and_then(Value::as_str) {
        return Ok(Some(name.to_lowercase()));
    }
    if let Some(types) = schema.get("type").and_then(Value::as_array) {
        if let Some(name) = types
            .iter()
            .filter_map(Value::as_str)
            .find(|name| *name != "null")
        {
            return Ok(Some(name.to_lowercase()));
        }
    }
    for key in ["anyOf", "oneOf", "allOf"] {
        if let Some(options) = schema.get(key).and_then(Value::as_array) {
            for item in options {
                if let Some(name) = schema_type(item, depth + 1, budget)? {
                    if !name.is_empty() && name != "null" {
                        return Ok(Some(name));
                    }
                }
            }
        }
    }
    Ok(None)
}
fn type_hint(
    capability: &Capability<'_>,
    param: &Parameter<'_>,
    budget: &mut usize,
) -> Result<String, Error> {
    if let Some(schema) = capability
        .raw
        .get("execution")
        .and_then(|v| v.get("inputSchema"))
        .and_then(|v| v.get("properties"))
        .and_then(|v| v.get(param.id))
    {
        if let Some(hint) = schema_type(schema, 0, budget)?.filter(|s| !s.is_empty()) {
            return Ok(hint);
        }
    }
    if let Some(input) = capability.inputs().find(|input| input.name == param.id) {
        // 主干先规范化端口 type：type > data_type > execution_type > any。
        return Ok(input.kind.to_lowercase());
    }
    Ok(param.data_type.unwrap_or(&param.widget).to_lowercase())
}
fn convert<'a>(value: &'a Value, hint: &str) -> Cow<'a, Value> {
    let Some(raw) = value.as_str() else {
        return Cow::Borrowed(value);
    };
    // 与 JS trim 一致，不使用 Rust trim 的不同空白字符集合。
    let trimmed = raw.trim_matches(crate::graph_param_coercion::initial_whitespace);
    if trimmed.is_empty() {
        return Cow::Borrowed(value);
    }
    if ["number", "integer", "float", "int", "double"]
        .iter()
        .any(|kind| hint.contains(kind))
    {
        let widget = Widget {
            name: "number",
            min: None,
            max: None,
        };
        if let Some(converted) = coerce(&widget, Output::Image(trimmed)) {
            if let Output::Number(number) = converted.as_output() {
                return Cow::Owned(Value::from(number));
            }
        }
        return Cow::Borrowed(value);
    }
    if ["boolean", "bool"].iter().any(|kind| hint.contains(kind)) {
        for (literal, boolean) in [("true", true), ("false", false)] {
            if trimmed.eq_ignore_ascii_case(literal) {
                return Cow::Owned(Value::Bool(boolean));
            }
        }
    }
    Cow::Borrowed(value)
}
impl<'a> Capability<'a> {
    /// 缺失值保留 None；secret/内部控制字段不进入可编辑、可发送的公共初始配置。
    /// 未转换的 JSON 继续借用目录，只有转换后的标量分配新值。
    pub fn initial_parameters(&self) -> Result<InitialParameters<'a>, Error> {
        let mut result = Vec::new();
        let mut budget = 4096;
        for (index, param) in self.parameters().enumerate() {
            if index >= 128 {
                return Err(Error::Capacity);
            }
            if param.secret || crate::graph_params::internal(param.id) {
                continue;
            }
            let value = match param.default {
                Some(value) if value.is_string() => {
                    Some(convert(value, &type_hint(self, &param, &mut budget)?))
                }
                value => value.map(Cow::Borrowed),
            };
            if let Some((_, previous)) = result.iter_mut().find(|(id, _)| *id == param.id) {
                *previous = value;
            } else {
                result.push((param.id, value));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "loom_capability_defaults_tests.rs"]
mod tests;
