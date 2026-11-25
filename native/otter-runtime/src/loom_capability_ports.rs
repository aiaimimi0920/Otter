//! Borrowed capability-port view aligned with artCapabilityNormalization/artPorts; no image-path resolution or resource reads.
use super::{text, Capability};
use serde_json::Value;
pub struct Port<'a> {
    pub name: &'a str,
    pub label: &'a str,
    pub kind: &'a str,
    pub execution_type: Option<&'a str>,
    pub data_type: Option<&'a str>,
    pub widget: Option<&'a str>,
    pub default: Option<&'a Value>,
    pub default_visible: Option<bool>,
    pub expose_port: Option<bool>,
    pub required: Option<bool>,
}
fn port(value: &Value) -> Option<Port<'_>> {
    let name = text(value, "name")?;
    let data_type = text(value, "data_type");
    let execution_type = text(value, "execution_type");
    Some(Port {
        name,
        label: text(value, "label").unwrap_or(name),
        kind: text(value, "type")
            .or(data_type)
            .or(execution_type)
            .unwrap_or("any"),
        execution_type,
        data_type,
        widget: text(value, "widget"),
        default: value.get("default"),
        default_visible: value["defaultVisible"].as_bool(),
        expose_port: value["exposePort"].as_bool(),
        required: value["required"].as_bool(),
    })
}
impl Port<'_> {
    pub fn is_image_like(&self) -> bool {
        [Some(self.kind), self.execution_type, self.data_type]
            .into_iter()
            .flatten()
            .any(|value| value.to_lowercase().contains("image"))
            || ["image", "input_image", "source_image", "reference_image"]
                .iter()
                .any(|name| self.name.eq_ignore_ascii_case(name))
    }
    pub fn is_primary_image(&self) -> bool {
        ["input", "input_image", "image"]
            .iter()
            .any(|name| self.name.eq_ignore_ascii_case(name))
    }
}
impl Capability<'_> {
    /// Use only the current single-port declaration; show by default, and parameter-value defaults do not affect visibility.
    pub fn port_default_visible(&self, name: &str, input: bool) -> bool {
        let local = if input {
            self.inputs().find(|p| p.name == name)
        } else {
            self.outputs().find(|p| p.name == name)
        };
        local.and_then(|p| p.default_visible).unwrap_or(true)
    }
    /// Distinguish Some(empty) from missing; the baseline uses caller fallback only when inputs is missing.
    pub fn has_input_definitions(&self) -> bool {
        self.raw.get("inputs").is_some_and(Value::is_array)
    }
    pub fn inputs(&self) -> impl Iterator<Item = Port<'_>> {
        self.raw
            .get("inputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(port)
    }
    pub fn outputs(&self) -> impl Iterator<Item = Port<'_>> {
        self.raw
            .get("outputs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(port)
    }
    pub fn visible_inputs(&self) -> impl Iterator<Item = Port<'_>> {
        self.inputs().filter(|port| {
            let matching_param = self
                .raw
                .get("parameters")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|param| text(param, "id") == Some(port.name));
            !matching_param
                || (port.is_image_like()
                    && (port.is_primary_image() || port.expose_port == Some(true)))
        })
    }
}
#[cfg(test)]
#[path = "loom_capability_ports_tests.rs"]
mod tests;
