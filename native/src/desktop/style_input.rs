//! 原生工具属性入口。模态对话框期间不持有模型借用，返回后重新检查 HWND。
use super::context_menu as menu;
use super::State;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::InvalidateRect,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

/// 样式命令集中到属性 owner；未识别命令无副作用，菜单的统一收尾仍由调用方执行。
pub(super) unsafe fn execute(
    hwnd: HWND,
    state: &State,
    command: usize,
) -> windows::core::Result<bool> {
    match command {
        menu::COLOR => choose_color(hwnd, state, false),
        menu::FILL_COLOR => choose_color(hwnd, state, true),
        super::screen_color::STROKE | super::screen_color::FILL => {
            super::screen_color::start(hwnd, state, command == super::screen_color::FILL)?
        }
        menu::WIDTH_DOWN => adjust_width(state, -1.0),
        menu::WIDTH_UP => adjust_width(state, 1.0),
        menu::RADIUS_DOWN => adjust_radius(hwnd, state, -1.0),
        menu::RADIUS_UP => adjust_radius(hwnd, state, 1.0),
        0x44 => cycle_dash(state),
        0x21 => adjust_sides(state, 1),
        0x22 => adjust_sides(state, -1),
        0x4F => cycle_opacity(state),
        0x46 => toggle_fill(state),
        _ => return Ok(false),
    }
    Ok(true)
}
pub(super) unsafe fn choose_color(hwnd: HWND, state: &State, fill: bool) {
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let (stamp, tool) = {
        let canvas = state.canvas.borrow();
        (canvas.editor.document_stamp(), canvas.tool)
    };
    let style = state.canvas.borrow().editor.style();
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    let initial = if fill {
        style.fill().unwrap_or(style.color())
    } else {
        style.color()
    };
    let Some(color) = super::color_dialog::choose(hwnd, initial) else {
        return;
    };
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let mut canvas = state.canvas.borrow_mut();
    if !stamp.matches(&canvas.editor) || canvas.tool != tool || canvas.editor.style() != style {
        return;
    }
    let style = canvas.editor.style();
    canvas.editor.set_style(if fill {
        style.with_fill(Some(color))
    } else {
        style.with_color(color)
    });
}

pub(super) fn adjust_width(state: &State, increment: f32) {
    let mut canvas = state.canvas.borrow_mut();
    let style = canvas.editor.style();
    if let Some(style) = width_style(canvas.tool, style, increment) {
        canvas.editor.set_style(style);
    }
}

pub(super) fn toggle_fill(state: &State) {
    let mut canvas = state.canvas.borrow_mut();
    let style = canvas.editor.style();
    let fill = if style.fill().is_some() {
        None
    } else {
        Some(style.color())
    };
    canvas.editor.set_style(style.with_fill(fill));
}

pub(super) unsafe fn adjust_radius(hwnd: HWND, state: &State, increment: f32) {
    if !increment.is_finite() {
        return;
    }
    let blur = {
        let canvas = state.canvas.borrow();
        canvas.tool == crate::editor::Tool::Blur
            || super::blur_input::selected_strength(&canvas).is_some()
    };
    if blur {
        super::blur_input::adjust_strength(hwnd, state, increment);
        return;
    }
    let mut canvas = state.canvas.borrow_mut();
    if canvas.tool == crate::editor::Tool::Serial {
        let radius =
            (canvas.tool_profiles.serial_radius() as f32 + increment).clamp(8.0, 999.0) as u16;
        canvas.tool_profiles.set_serial_radius(radius);
        return;
    }
    let style = canvas.editor.style();
    let radius = (style.corner_radius() + increment).clamp(0.0, 256.0);
    if canvas.tool == crate::editor::Tool::Mosaic {
        if let Some(style) = mosaic_style(style, increment) {
            canvas.editor.set_style(style);
        }
        return;
    }
    canvas
        .editor
        .set_style(style.with_corner_radius(radius).unwrap());
}

fn width_style(
    tool: crate::editor::Tool,
    style: crate::stroke_style::StrokeStyle,
    increment: f32,
) -> Option<crate::stroke_style::StrokeStyle> {
    if !increment.is_finite() {
        return None;
    }
    let (min, max) = super::numeric_style::Kind::width(tool).bounds();
    style.with_width((style.width() + increment).clamp(min, max))
}
fn mosaic_style(
    style: crate::stroke_style::StrokeStyle,
    increment: f32,
) -> Option<crate::stroke_style::StrokeStyle> {
    if !increment.is_finite() {
        return None;
    }
    let (min, max) = super::numeric_style::Kind::MosaicCell.bounds();
    style.with_mosaic_cell_value((style.mosaic_cell() + increment).clamp(min, max))
}

pub(super) fn cycle_opacity(state: &State) {
    let mut canvas = state.canvas.borrow_mut();
    let style = canvas.editor.style();
    let opacity = if style.opacity() >= 1.0 {
        0.25
    } else {
        (style.opacity() + 0.25).min(1.0)
    };
    canvas
        .editor
        .set_style(style.with_opacity(opacity).unwrap());
}

pub(super) fn cycle_dash(state: &State) {
    let mut canvas = state.canvas.borrow_mut();
    let style = canvas.editor.style();
    canvas
        .editor
        .set_style(style.with_dash(style.dash().next()));
}

pub(super) fn adjust_sides(state: &State, increment: i8) {
    let mut canvas = state.canvas.borrow_mut();
    let style = canvas.editor.style();
    let sides = (style.sides() as i16 + increment as i16).clamp(3, 12) as u8;
    canvas.editor.set_style(style.with_sides(sides));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::Tool, stroke_style::StrokeStyle};
    #[test]
    fn width_controls_share_tool_specific_limits_and_keep_fractions() {
        let style = StrokeStyle::default().with_width(199.5).unwrap();
        for tool in [Tool::Blur, Tool::Mosaic] {
            assert_eq!(width_style(tool, style, 1.0).unwrap().width(), 200.0);
            assert_eq!(width_style(tool, style, -500.0).unwrap().width(), 4.0);
        }
        assert_eq!(width_style(Tool::Brush, style, 1.0).unwrap().width(), 200.5);
        assert_eq!(width_style(Tool::Eraser, style, 1.0).unwrap().width(), 96.0);
        assert!(width_style(Tool::Blur, style, f32::NAN).is_none());
    }
    #[test]
    fn mosaic_adjustment_does_not_truncate_fraction_or_change_other_fields() {
        let style = StrokeStyle::default().with_mosaic_cell_value(2.4).unwrap();
        let next = mosaic_style(style, 1.0).unwrap();
        assert_eq!(next.mosaic_cell(), 3.4);
        assert_eq!(next.width(), style.width());
        assert_eq!(next.color(), style.color());
        assert_eq!(mosaic_style(style, -50.0).unwrap().mosaic_cell(), 2.0);
        assert_eq!(mosaic_style(style, 500.0).unwrap().mosaic_cell(), 64.0);
        assert!(mosaic_style(style, f32::INFINITY).is_none());
    }
}
