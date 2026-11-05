//! 精确样式输入：校验有限数值、保留其他属性，模态返回后校验目标再应用。
use super::{canvas::Canvas, State};
use crate::{
    editor::{DocumentStamp, Tool},
    stroke_style::StrokeStyle,
};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsWindow};
pub(super) const WIDTH: usize = 0x1042;
pub(super) const OPACITY: usize = 0x1043;
pub(super) const MOSAIC: usize = 0x104b;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Width,
    EraserWidth,
    EffectWidth,
    MosaicCell,
    Opacity,
    BlurStrength,
    BlurDefaultStrength,
}
impl Kind {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Width | Self::EraserWidth | Self::EffectWidth => "精确粗细",
            Self::MosaicCell => "精确色块大小",
            Self::Opacity => "精确透明度",
            Self::BlurStrength => "选中模糊强度",
            Self::BlurDefaultStrength => "新建模糊强度",
        }
    }
    pub(super) fn hint(self) -> &'static str {
        match self {
            Self::Width => "请输入 1–256 的粗细，允许小数",
            Self::EraserWidth => "请输入 4–96 的橡皮擦粗细，允许小数",
            Self::EffectWidth => "请输入 4–200 的效果笔刷大小，允许小数",
            Self::MosaicCell => "请输入 2–64 的色块大小，允许小数",
            Self::Opacity => "请输入 0–100 的百分比，允许小数",
            Self::BlurStrength => "请输入 1–64 的模糊强度，允许小数",
            Self::BlurDefaultStrength => "请输入 2–64 的模糊强度，允许小数",
        }
    }
    pub(super) fn parse(self, text: &str) -> Option<f32> {
        if text.len() > 32 {
            return None;
        }
        let value: f32 = text.trim().parse().ok()?;
        let (min, max) = self.bounds();
        (value.is_finite() && value >= min && value <= max).then_some(value)
    }
    pub(super) fn width(tool: Tool) -> Self {
        match tool {
            Tool::Eraser => Self::EraserWidth,
            Tool::Blur | Tool::Mosaic => Self::EffectWidth,
            _ => Self::Width,
        }
    }
    pub(super) fn bounds(self) -> (f32, f32) {
        match self {
            Self::Width => (1.0, 256.0),
            Self::EraserWidth => (4.0, 96.0),
            Self::EffectWidth => (4.0, 200.0),
            Self::MosaicCell => (2.0, 64.0),
            Self::Opacity => (0.0, 100.0),
            Self::BlurStrength => (1.0, 64.0),
            Self::BlurDefaultStrength => (2.0, 64.0),
        }
    }
    fn initial(self, style: StrokeStyle) -> Option<String> {
        match self {
            Self::Opacity => Some((style.opacity() * 100.0).to_string()),
            Self::Width | Self::EraserWidth | Self::EffectWidth => Some(style.width().to_string()),
            Self::MosaicCell => Some(style.mosaic_cell().to_string()),
            Self::BlurStrength | Self::BlurDefaultStrength => None,
        }
    }
}
struct Pending {
    stamp: DocumentStamp,
    tool: Tool,
    style: StrokeStyle,
    kind: Kind,
}
impl Pending {
    fn capture(canvas: &Canvas, command: usize) -> Option<Self> {
        let kind = match command {
            WIDTH => Kind::width(canvas.tool),
            MOSAIC if canvas.tool == Tool::Mosaic => Kind::MosaicCell,
            OPACITY => Kind::Opacity,
            _ => return None,
        };
        Some(Self {
            stamp: canvas.editor.document_stamp(),
            tool: canvas.tool,
            style: canvas.editor.style(),
            kind,
        })
    }
    fn apply(self, canvas: &mut Canvas, text: &str) -> bool {
        if !self.stamp.matches(&canvas.editor)
            || self.tool != canvas.tool
            || self.style != canvas.editor.style()
        {
            return false;
        }
        let Some(value) = self.kind.parse(text) else {
            return false;
        };
        let style = match self.kind {
            Kind::Opacity => self.style.with_opacity(value / 100.0),
            Kind::Width | Kind::EraserWidth | Kind::EffectWidth => self.style.with_width(value),
            Kind::MosaicCell => self.style.with_mosaic_cell_value(value),
            Kind::BlurStrength | Kind::BlurDefaultStrength => return false,
        };
        let Some(style) = style else {
            return false;
        };
        if style == self.style {
            return false;
        }
        canvas.editor.set_style(style);
        true
    }
}
pub(super) unsafe fn show(hwnd: HWND, state: &State, command: usize) {
    if !super::tool_panel_availability::Availability::capture(state)
        .is_some_and(|value| value.allows(command))
    {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let Some(pending) = Pending::capture(&state.canvas.borrow(), command) else {
        return;
    };
    let Some(initial) = pending.kind.initial(pending.style) else {
        return;
    };
    let value = super::shape_dialog::show_numeric(hwnd, initial, pending.kind);
    if IsWindow(Some(hwnd)).as_bool() {
        if let Some(value) = value {
            pending.apply(&mut state.canvas.borrow_mut(), &value);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_mosaic_cell_reaches_stroke_history_and_tool_profile_without_rounding() {
        let mut canvas = Canvas::new(Some(super::super::image::Image {
            width: 8,
            height: 8,
            pixels: vec![255; 256].into(),
        }))
        .unwrap();
        canvas.select_tool(Tool::Mosaic);
        let width = canvas.editor.style().width();
        assert!(Pending::capture(&canvas, MOSAIC)
            .unwrap()
            .apply(&mut canvas, "2.4"));
        assert_eq!(canvas.editor.style().width(), width);
        let point = crate::viewport::Point {
            x: canvas.viewport.origin.x + 4.0 * canvas.viewport.scale,
            y: canvas.viewport.origin.y + 4.0 * canvas.viewport.scale,
        };
        assert!(canvas.pointer_down(point));
        canvas.pointer_up(point);
        assert_eq!(canvas.editor.elements()[0].mosaic_cell, 2.4);
        assert!(canvas.editor.undo());
        assert!(canvas.editor.redo());
        assert_eq!(canvas.editor.elements()[0].mosaic_cell, 2.4);
        canvas.select_tool(Tool::Brush);
        canvas.select_tool(Tool::Mosaic);
        assert_eq!(canvas.editor.style().mosaic_cell(), 2.4);
        assert!(!Pending::capture(&canvas, MOSAIC)
            .unwrap()
            .apply(&mut canvas, "64.1"));
        let pending = Pending::capture(&canvas, MOSAIC).unwrap();
        canvas.select_tool(Tool::Blur);
        assert!(!pending.apply(&mut canvas, "5.4"));
        assert!(Pending::capture(&canvas, MOSAIC).is_none());
    }
    #[test]
    fn accepts_decimal_boundaries_and_rejects_nonfinite_or_out_of_range_values() {
        for (kind, good, bad) in [
            (Kind::Width, "1.25", "0.99"),
            (Kind::EraserWidth, "96", "96.1"),
            (Kind::EffectWidth, "199.5", "200.1"),
            (Kind::MosaicCell, "2.4", "1.99"),
            (Kind::Opacity, "12.5", "100.1"),
            (Kind::BlurStrength, "2.4", "64.1"),
        ] {
            assert!(kind.parse(good).is_some());
            assert!(kind.parse(bad).is_none());
            for bad in ["", "NaN", "inf", "-inf", "1e100", "1,5", "50%"] {
                assert!(kind.parse(bad).is_none());
            }
        }
        assert_eq!(Kind::Opacity.parse(" 0 "), Some(0.0));
        assert_eq!(Kind::Opacity.parse("100"), Some(100.0));
        assert!(Kind::Width.parse(&"1".repeat(33)).is_none());
    }
    #[test]
    fn pending_change_preserves_other_fields_and_rejects_stale_document_tool_or_style() {
        let mut canvas = Canvas::new(None).unwrap();
        let before = canvas.editor.style();
        assert!(Pending::capture(&canvas, WIDTH)
            .unwrap()
            .apply(&mut canvas, "12.5"));
        assert_eq!(canvas.editor.style().width(), 12.5);
        assert_eq!(canvas.editor.style().color(), before.color());
        assert!(Pending::capture(&canvas, OPACITY)
            .unwrap()
            .apply(&mut canvas, "25"));
        assert_eq!(canvas.editor.style().opacity(), 0.25);
        assert_eq!(canvas.editor.style().width(), 12.5);
        let pending = Pending::capture(&canvas, WIDTH).unwrap();
        canvas.editor = canvas.editor.export_snapshot();
        assert!(!pending.apply(&mut canvas, "20"));
        let pending = Pending::capture(&canvas, WIDTH).unwrap();
        canvas.select_tool(Tool::Brush);
        assert!(!pending.apply(&mut canvas, "20"));
        let pending = Pending::capture(&canvas, WIDTH).unwrap();
        canvas
            .editor
            .set_style(before.with_color(crate::stroke_style::Color(1, 2, 3)));
        assert!(!pending.apply(&mut canvas, "20"));
    }
}
