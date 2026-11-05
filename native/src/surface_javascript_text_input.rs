//! 私有文本输入 carrier；只有有界值/目标/阶段，不含远端 action、凭据或 HWND。
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum InputPhase {
    Input,
    Change,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_native_input_carrier_rejects_unbounded_or_non_single_line_wire() {
        let mut input = TextInput {
            target: "dom:2".into(),
            phase: InputPhase::Input,
            value: "中文🙂".into(),
        };
        assert!(input.validate().is_ok());
        for value in [
            "x\n".into(),
            "x\r".into(),
            "x\0".into(),
            "x".repeat(48 * 1024 + 1),
        ] {
            input.value = value;
            assert!(input.validate().is_err());
        }
        input.value.clear();
        input.target = "not safe".into();
        assert!(input.validate().is_err());
        for value in [
            r#"{"target":"dom:2","phase":"keydown","value":""}"#,
            r#"{"target":"dom:2","phase":"input","value":"","action":"fake"}"#,
        ] {
            assert!(serde_json::from_str::<TextInput>(value).is_err());
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextInput {
    pub target: String,
    pub phase: InputPhase,
    pub value: String,
}
impl TextInput {
    pub fn validate(&self) -> io::Result<()> {
        if !crate::surface_javascript_event::safe(&self.target)
            || self.value.len() > crate::surface_input_draft::MAX_VALUE_BYTES
            || self.value.contains(['\0', '\r', '\n'])
        {
            return Err(super::invalid());
        }
        Ok(())
    }
}
