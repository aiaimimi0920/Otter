//! 新建模糊工具的精确强度；只改默认配置和准备缓存，不修改已经提交的标注。
use super::{canvas::Canvas, numeric_style::Kind, State};
use crate::editor::{DocumentStamp, Tool};
use windows::{
    core::w,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x104a;
struct Pending {
    stamp: DocumentStamp,
    strength: f32,
}
impl Pending {
    fn capture(canvas: &Canvas) -> Option<Self> {
        (canvas.tool == Tool::Blur && canvas.image.is_some()).then(|| Self {
            stamp: canvas.editor.document_stamp(),
            strength: canvas.tool_profiles.blur_strength(),
        })
    }
    fn apply(self, canvas: &mut Canvas, text: &str) -> bool {
        if !self.stamp.matches(&canvas.editor)
            || canvas.tool != Tool::Blur
            || canvas.tool_profiles.blur_strength() != self.strength
        {
            return false;
        }
        let Some(value) = Kind::BlurDefaultStrength.parse(text) else {
            return false;
        };
        if value == self.strength || !canvas.tool_profiles.set_blur_strength(value) {
            return false;
        }
        canvas.cancel_gesture();
        canvas.blur_ready = None;
        true
    }
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> windows::core::Result<()> {
    let enabled = Pending::capture(&state.canvas.borrow()).is_some();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        COMMAND,
        w!("新建模糊强度…"),
    )
}
pub(super) unsafe fn show(hwnd: HWND, state: &State) {
    if !super::tool_panel_availability::Availability::capture(state)
        .is_some_and(|value| value.allows(COMMAND))
    {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let Some(pending) = Pending::capture(&state.canvas.borrow()) else {
        return;
    };
    let value = super::shape_dialog::show_numeric(
        hwnd,
        pending.strength.to_string(),
        Kind::BlurDefaultStrength,
    );
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    if let Some(value) = value {
        let changed = pending.apply(&mut state.canvas.borrow_mut(), &value);
        if changed {
            super::blur_input::start(hwnd, state);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Canvas {
        let mut canvas = Canvas::new(Some(super::super::image::Image {
            width: 4,
            height: 4,
            pixels: vec![255; 64].into(),
        }))
        .unwrap();
        canvas.select_tool(Tool::Blur);
        canvas
    }
    #[test]
    fn fractional_default_preserves_document_and_rejects_stale_input() {
        let mut canvas = fixture();
        let stamp = canvas.editor.document_stamp();
        assert!(Pending::capture(&canvas).unwrap().apply(&mut canvas, "2.4"));
        assert_eq!(canvas.tool_profiles.blur_strength(), 2.4);
        assert!(stamp.matches(&canvas.editor));
        for text in ["2.4", "1.99", "64.01", "NaN", "inf"] {
            assert!(!Pending::capture(&canvas).unwrap().apply(&mut canvas, text));
        }
        let pending = Pending::capture(&canvas).unwrap();
        canvas.tool_profiles.set_blur_strength(4.4);
        assert!(!pending.apply(&mut canvas, "5.4"));
        let pending = Pending::capture(&canvas).unwrap();
        canvas.editor = canvas.editor.export_snapshot();
        assert!(!pending.apply(&mut canvas, "5.4"));
        let pending = Pending::capture(&canvas).unwrap();
        canvas.select_tool(Tool::Brush);
        assert!(!pending.apply(&mut canvas, "5.4"));
    }
}
