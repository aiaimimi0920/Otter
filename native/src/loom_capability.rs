//! 能力目录的借用视图，对齐主干 artCapabilityNormalization；不复制目录或凭据默认值。
use serde_json::Value;
use std::borrow::Cow;
#[path = "loom_capability_defaults.rs"]
mod defaults;
pub use defaults::{Error as DefaultsError, InitialParameters};
#[path = "loom_capability_ports.rs"]
mod ports;
#[path = "loom_capability_runtime.rs"]
mod runtime;
pub use ports::Port;

pub struct Capability<'a> {
    raw: &'a Value,
    pub id: &'a str,
    pub label: &'a str,
    pub enabled: bool,
}
pub struct Parameter<'a> {
    raw: &'a Value,
    pub id: &'a str,
    pub label: &'a str,
    pub widget: String,
    pub data_type: Option<&'a str>,
    pub default: Option<&'a Value>,
    pub secret: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub multiline: bool,
    pub group: Option<&'a str>,
    pub required: Option<bool>,
    pub disabled: Option<bool>,
}
pub struct OptionItem<'a> {
    pub value: &'a Value,
    pub label: Cow<'a, str>,
}
fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64().filter(|n| n.is_finite())
}
fn secret(value: &Value) -> bool {
    value["secret"].as_bool() == Some(true)
        || ["type", "data_type"]
            .iter()
            .any(|key| text(value, key).is_some_and(|s| s.eq_ignore_ascii_case("secret")))
}
fn option(value: &Value) -> Option<OptionItem<'_>> {
    let scalar = if value.is_object() {
        value.get("value")?
    } else {
        value
    };
    if !matches!(scalar, Value::String(_) | Value::Number(_) | Value::Bool(_)) {
        return None;
    }
    let label = text(value, "label")
        .map(Cow::Borrowed)
        .unwrap_or_else(|| match scalar.as_str() {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned(scalar.to_string()),
        });
    Some(OptionItem {
        value: scalar,
        label,
    })
}
fn options(raw: &Value) -> impl Iterator<Item = OptionItem<'_>> {
    raw.get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(option)
}
fn widget(raw: &Value, data_type: Option<&str>) -> String {
    if let Some(explicit) = text(raw, "widget") {
        return explicit.to_lowercase();
    }
    if options(raw).next().is_some() {
        return "select".into();
    }
    match data_type.unwrap_or("").to_lowercase().as_str() {
        "boolean" | "bool" => "checkbox",
        "number" | "integer" | "int" | "float" | "double" => "number",
        "enum" | "select" => "select",
        "json" | "object" | "array" => "textarea",
        "path" | "directory" | "folder" => "path",
        name if name.contains("image") => "image_link",
        _ => "text",
    }
    .into()
}
impl<'a> Capability<'a> {
    pub fn new(raw: &'a Value) -> Option<Self> {
        if !raw.is_object() {
            return None;
        }
        let id = text(raw, "id")?;
        Some(Self {
            raw,
            id,
            label: text(raw, "label").unwrap_or(id),
            // 与主干一致：只有 JSON boolean false 禁用，不能把缺失当 false。
            enabled: raw["enabled"].as_bool() != Some(false),
        })
    }
    pub fn parameters(&self) -> impl Iterator<Item = Parameter<'a>> + '_ {
        let declarations = self.raw.get("parameters").and_then(Value::as_array);
        declarations.into_iter().flatten().filter_map(move |raw| {
            let id = text(raw, "id")?;
            // 同名声明任意一项为 secret 都禁止默认值外泄，不能被后续公共声明覆盖。
            let secret = declarations
                .into_iter()
                .flatten()
                .any(|other| text(other, "id") == Some(id) && secret(other));
            let data_type = text(raw, "data_type").or_else(|| text(raw, "type"));
            let widget = widget(raw, data_type);
            let default = if secret {
                None
            } else {
                raw.get("default")
                    .or_else(|| self.raw.get("defaults")?.as_object()?.get(id))
            };
            Some(Parameter {
                raw,
                id,
                label: text(raw, "label").unwrap_or(id),
                data_type,
                default,
                secret,
                min: finite(raw, "min"),
                max: finite(raw, "max"),
                step: finite(raw, "step"),
                multiline: raw["multiline"].as_bool().unwrap_or(widget == "textarea"),
                widget,
                group: text(raw, "group"),
                required: raw["required"].as_bool(),
                disabled: raw["disabled"].as_bool(),
            })
        })
    }
}
impl Parameter<'_> {
    pub fn options(&self) -> impl Iterator<Item = OptionItem<'_>> {
        options(self.raw)
    }
}

#[cfg(test)]
#[path = "loom_capability_tests.rs"]
mod tests;
