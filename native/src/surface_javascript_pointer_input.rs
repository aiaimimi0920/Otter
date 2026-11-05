//! Canvas 鼠标私有输入；坐标为客户区 DIP，不携带 HWND、远端动作或捕获权限。
use super::invalid;
use serde::{Deserialize, Serialize};
use std::io;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PointerPhase {
    Pointerdown,
    Pointermove,
    Pointerup,
    Pointercancel,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PointerInput {
    pub target: String,
    pub phase: PointerPhase,
    pub client: [f64; 2],
    /// bit0 ctrl、bit1 shift、bit2 alt、bit3 meta；目前只接左键 mouse/pointerId=1。
    pub modifiers: u8,
    /// 原 HWND 在 up 时按同命中/系统拖动阈值确认；click 与 up 共用 FIFO 槽。
    pub click: bool,
}
impl PointerInput {
    pub fn validate(&self) -> io::Result<()> {
        if !crate::surface_javascript_event::safe(&self.target)
            || self.modifiers > 15
            || self
                .client
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 131_072.)
            || (self.click && self.phase != PointerPhase::Pointerup)
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn terminal(&self) -> bool {
        matches!(
            self.phase,
            PointerPhase::Pointerup | PointerPhase::Pointercancel
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_pointer_wire_rejects_nonfinite_bounds_unknown_authority_and_click_phase() {
        let mut input = PointerInput {
            target: "dom:2".into(),
            phase: PointerPhase::Pointermove,
            client: [-1., 2.5],
            modifiers: 15,
            click: false,
        };
        assert!(input.validate().is_ok());
        for value in [f64::NAN, f64::INFINITY, -131_073., 131_073.] {
            input.client[0] = value;
            assert!(input.validate().is_err());
        }
        input.client[0] = 0.;
        input.click = true;
        assert!(input.validate().is_err());
        input.phase = PointerPhase::Pointerup;
        assert!(input.validate().is_ok());
        input.modifiers = 16;
        assert!(input.validate().is_err());
        for json in [
            r#"{"target":"dom:2","phase":"pointerdown","client":[0,1],"modifiers":0,"click":false,"hwnd":1}"#,
            r#"{"target":"dom:2","phase":"pointerover","client":[0,1],"modifiers":0,"click":false}"#,
        ] {
            assert!(serde_json::from_str::<PointerInput>(json).is_err());
        }
    }
}
