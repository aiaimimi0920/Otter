//! 字段 input/change 编码，复用既有不可变事件/提交预算；不授权或发送动作。
use crate::{
    surface_click_event::{self, ClickEvent, Declaration},
    surface_live_binding::Binding,
    surface_scene::{Kind, Node},
};

#[derive(Clone, Copy)]
pub enum Phase {
    Input,
    Change,
}
#[derive(Clone, Copy)]
pub enum Value<'a> {
    Text(&'a str),
    Number(Option<f64>),
    Boolean(bool),
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Kind,
    Value,
    Encoding(surface_click_event::Error),
}

pub fn encode(
    node: &Node,
    binding: &Binding,
    revision: u64,
    phase: Phase,
    value: Value<'_>,
) -> Result<Option<ClickEvent>, Error> {
    encode_using(node, phase, value, |declaration| {
        surface_click_event::encode(binding, revision, declaration)
    })
}
/// Wall 与普通宿主只共享 wire 编码；此身份不授普通 Surface 挂载权。
pub(crate) fn encode_identity(
    node: &Node,
    identity: surface_click_event::Identity<'_>,
    revision: u64,
    phase: Phase,
    value: Value<'_>,
) -> Result<Option<(String, Vec<u8>)>, Error> {
    encode_using(node, phase, value, |declaration| {
        surface_click_event::encode_identity(identity, revision, declaration)
    })
}
fn encode_using<T>(
    node: &Node,
    phase: Phase,
    value: Value<'_>,
    encoder: impl FnOnce(Declaration<'_>) -> Result<T, surface_click_event::Error>,
) -> Result<Option<T>, Error> {
    if !matches!(
        node.kind(),
        Kind::Input | Kind::Textarea | Kind::Number | Kind::Switch | Kind::Select | Kind::Slider
    ) {
        return Err(Error::Kind);
    }
    // Switch/Select 的既有合同只产生 change，不伪造连续事件或文本 draft。
    if matches!(node.kind(), Kind::Switch | Kind::Select) && matches!(phase, Phase::Input) {
        return Ok(None);
    }
    if !node.visible() || node.props().get("disabled") == Some(&serde_json::Value::Bool(true)) {
        return Ok(None);
    }
    let (event, class) = match phase {
        Phase::Input => ("input", "continuous"),
        Phase::Change => ("change", "commit"),
    };
    let Some(action) = node.events().get(event) else {
        return Ok(None);
    };
    let value = match (node.kind(), value) {
        (Kind::Switch, Value::Boolean(value)) => serde_json::Value::Bool(value),
        (Kind::Input | Kind::Textarea | Kind::Select, Value::Text(text)) => {
            if text.len() > crate::surface_input_draft::MAX_VALUE_BYTES || text.contains('\0') {
                return Err(Error::Value);
            }
            serde_json::Value::String(text.to_owned())
        }
        (Kind::Slider, Value::Number(None)) => return Err(Error::Value),
        (Kind::Number | Kind::Slider, Value::Number(number)) => match number {
            // HTML valueAsNumber 的空/无效值经 JSON 编码为 null，不伪造为零。
            None => serde_json::Value::Null,
            Some(number) => serde_json::Number::from_f64(number)
                .map(serde_json::Value::Number)
                .ok_or(Error::Value)?,
        },
        _ => return Err(Error::Value),
    };
    let payload = serde_json::json!({"value":value});
    encoder(Declaration {
        node_id: node.id(),
        event,
        action,
        class,
        modifiers: None,
        payload: &payload,
    })
    .map(Some)
    .map_err(Error::Encoding)
}
#[cfg(test)]
#[path = "surface_input_event_tests.rs"]
mod tests;
