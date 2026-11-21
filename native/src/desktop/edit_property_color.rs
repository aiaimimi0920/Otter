//! HSV 弹层到标注样式的适配；主题只管理草稿，应用拥有目标、历史和会话调色板。
use super::*;
use crate::stroke_style::{Color, StrokeStyle};
use neuro_mini_ui::{ColorAction, ColorIcons, ColorOptions, ColorResult, Icon, Rgba};
fn initial(style: StrokeStyle, fill: bool) -> Rgba {
    let Color(r, g, b) = if fill {
        style.fill().unwrap_or(style.color())
    } else {
        style.color()
    };
    let alpha = if fill {
        if style.fill().is_some() {
            style.paint_alpha().fill()
        } else {
            0.0
        }
    } else {
        style.paint_alpha().stroke()
    };
    Rgba {
        r,
        g,
        b,
        a: (alpha * 255.0).round() as u8,
    }
}
fn icons() -> ColorIcons {
    ColorIcons {
        eyedropper: Icon::svg(include_str!("../../assets/edit-strip/ColorEyedropper.svg")),
        remove: Icon::svg(include_str!("../../assets/edit-strip/ColorRemove.svg")),
        apply: Icon::svg(include_str!("../../assets/edit-strip/ColorApply.svg")),
    }
}
pub(super) unsafe fn execute(
    owner: HWND,
    state: &State,
    id: u16,
    anchor: RECT,
    target: &MenuTarget,
) -> Result<()> {
    let options = {
        let c = state.canvas.borrow();
        let Some(v) = fields::view(&c) else {
            return Ok(());
        };
        ColorOptions {
            initial: initial(v.style, id == FILL),
            palette: state.edit_palette.borrow().colors.clone(),
            screen_pick: v.selected.is_none(),
            icons: icons(),
        }
    };
    let result = {
        let Some(_modal) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(());
        };
        neuro_mini_ui::track_color(state.tool_panel.get(), anchor, options)?
    };
    accept(owner, state, id, target, result)
}
unsafe fn accept(
    owner: HWND,
    state: &State,
    id: u16,
    target: &MenuTarget,
    result: ColorResult,
) -> Result<()> {
    // 先核凭证，再独立处理删除默认值；删除自身改变 default style，不能据此误拒随后的“应用”。
    let valid = menus::valid_target(owner, state, target);
    state
        .edit_palette
        .borrow_mut()
        .record(state, result.palette, result.removed);
    if !valid {
        return Ok(());
    }
    match result.action {
        ColorAction::Apply(color) => {
            apply::rgba(&mut state.canvas.borrow_mut(), id == FILL, color);
            state.edit_palette.borrow_mut().active = color;
        }
        ColorAction::ScreenPick => super::super::screen_color::start_palette(owner, state)?,
        ColorAction::Cancel => (),
    }
    Ok(())
}
#[cfg(test)]
#[path = "edit_property_color_tests.rs"]
mod tests;
