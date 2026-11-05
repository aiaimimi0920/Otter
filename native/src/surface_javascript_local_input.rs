//! 私有本地输入联合：文本阶段与受限 Enter 边沿分型，不包含 HWND/动作权限。
use super::{invalid, HoverInput, PointerInput, TextInput};
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "event",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum LocalInput {
    Text(TextInput),
    Keyboard(KeyboardInput),
    Pointer(PointerInput),
    Hover(HoverInput),
}
impl LocalInput {
    pub fn validate(&self) -> io::Result<()> {
        match self {
            Self::Text(input) => input.validate(),
            Self::Keyboard(input) => input.validate(),
            Self::Pointer(input) => input.validate(),
            Self::Hover(input) => input.validate(),
        }
    }
    pub fn keyboard(&self) -> bool {
        matches!(self, Self::Keyboard(_))
    }
}
impl From<TextInput> for LocalInput {
    fn from(value: TextInput) -> Self {
        Self::Text(value)
    }
}
impl From<KeyboardInput> for LocalInput {
    fn from(value: KeyboardInput) -> Self {
        Self::Keyboard(value)
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum KeyPhase {
    Keydown,
    Keyup,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct KeyboardInput {
    pub target: String,
    pub phase: KeyPhase,
    /// bit0 keyup, bit1 repeat, bit2 ctrl, bit3 shift, bit4 alt, bit5 meta, bit6 numpad。
    pub flags: u32,
}
impl KeyboardInput {
    pub fn validate(&self) -> io::Result<()> {
        if !crate::surface_javascript_event::safe(&self.target)
            || self.flags > 127
            || (self.flags & 1 != 0) != matches!(self.phase, KeyPhase::Keyup)
            || self.flags & 3 == 3
        {
            return Err(invalid());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_native_enter_wire_rejects_unknown_flags_phase_privilege_and_text_confusion() {
        let mut input = KeyboardInput {
            target: "dom:2".into(),
            phase: KeyPhase::Keydown,
            flags: 126,
        };
        assert!(input.validate().is_ok());
        for flags in [1, 3, 127, 128, u32::MAX] {
            input.flags = flags;
            assert!(input.validate().is_err());
        }
        input.flags = 1;
        input.phase = KeyPhase::Keyup;
        assert!(input.validate().is_ok());
        for target in ["not safe".into(), "x".repeat(257)] {
            input.target = target;
            assert!(input.validate().is_err());
        }
        for json in [
            r#"{"kind":"keyboard","event":{"target":"dom:2","phase":"input","flags":0}}"#,
            r#"{"kind":"keyboard","event":{"target":"dom:2","phase":"keydown","flags":0,"hwnd":1}}"#,
            r#"{"kind":"text","event":{"target":"dom:2","value":"","phase":"keydown"}}"#,
            r#"{"kind":"keyboard","event":{"target":"dom:2","phase":"keydown","flags":0},"action":"fake"}"#,
            r#"{"kind":"keyboard","event":{"target":"dom:2","phase":"keydown","flags":0,"value":"old"}}"#,
        ] {
            assert!(serde_json::from_str::<LocalInput>(json).is_err());
        }
    }
}
