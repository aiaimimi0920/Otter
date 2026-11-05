//! 非捕获鼠标位置；None 是离开文档，不携带窗口/远端权限或宿主默认动作。
use super::invalid;
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HoverInput {
    pub target: Option<String>,
    pub client: [f64; 2],
    pub modifiers: u8,
}
impl HoverInput {
    pub fn validate(&self) -> io::Result<()> {
        if self
            .target
            .as_ref()
            .is_some_and(|id| !crate::surface_javascript_event::safe(id))
            || self.modifiers > 15
            || self
                .client
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 131_072.)
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
    fn javascript_hover_wire_bounds_and_no_window_privilege() {
        let mut input = HoverInput {
            target: Some("dom:2".into()),
            client: [131_072., -131_072.],
            modifiers: 15,
        };
        assert!(input.validate().is_ok());
        for x in [f64::NAN, f64::INFINITY, 131_072.1] {
            input.client[0] = x;
            assert!(input.validate().is_err());
        }
        input.client = [0., 0.];
        input.modifiers = 16;
        assert!(input.validate().is_err());
        input.modifiers = 0;
        for target in ["not safe".into(), "x".repeat(257)] {
            input.target = Some(target);
            assert!(input.validate().is_err());
        }
        input.target = None;
        assert!(input.validate().is_ok());
        for json in [
            r#"{"target":null,"client":[0,0],"modifiers":0,"hwnd":1}"#,
            r#"{"target":null,"client":[0,0],"modifiers":0,"action":"fake"}"#,
            r#"{"target":null,"client":[0],"modifiers":0}"#,
        ] {
            assert!(serde_json::from_str::<HoverInput>(json).is_err());
        }
    }
}
