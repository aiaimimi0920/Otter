//! Color/line-style/font selections commit under the same target evidence; no Canvas borrow during modals.
use super::*;
use crate::stroke_style::DashPattern;
use neuro_mini_ui::{Icon, MenuItem};
pub(super) fn valid_target(owner: HWND, state: &State, target: &MenuTarget) -> bool {
    state.lifetime.owns(owner) && unsafe { IsWindowVisible(state.tool_panel.get()).as_bool() } && {
        let c = state.canvas.borrow();
        target.matches(&c) && ready(state, &c)
    }
}
pub(super) unsafe fn execute(
    owner: HWND,
    state: &State,
    id: u16,
    target: &MenuTarget,
) -> Result<()> {
    let panel = state.tool_panel.get();
    let Some(anchor) = neuro_mini_ui::property_anchor(panel, id) else {
        return Ok(());
    };
    if matches!(id, STROKE | FILL) {
        return color_picker::execute(owner, state, id, anchor, target);
    }
    if id == FONT {
        return font_picker::execute(owner, state, anchor, target);
    }
    let initial = {
        let c = state.canvas.borrow();
        let Some(v) = fields::view(&c) else {
            return Ok(());
        };
        v.style
    };
    let (items, current, compact): (Vec<(String, Icon)>, usize, bool) = if id == SHAPE {
        let current = {
            let c = state.canvas.borrow();
            let Some(v) = fields::view(&c) else {
                return Ok(());
            };
            match v.tool {
                crate::editor::Tool::Magnifier => {
                    match v
                        .selected
                        .and_then(|i| c.editor.elements()[i].lens)
                        .unwrap_or(c.tool_profiles.lens)
                        .shape
                    {
                        crate::editor::LensShape::Rect => 0,
                        crate::editor::LensShape::Rounded => 1,
                        crate::editor::LensShape::Ellipse => 2,
                    }
                }
                crate::editor::Tool::SpotRounded => 1,
                crate::editor::Tool::SpotEllipse => 2,
                _ => 0,
            }
        };
        (
            ["tool-shape-rect", "tool-shape-round", "tool-shape-ellipse"]
                .into_iter()
                .enumerate()
                .map(|(i, key)| (crate::ui_text::text(key).to_owned(), fields::shape_icon(i)))
                .collect(),
            current,
            false,
        )
    } else {
        (
            ["edit-solid-line", "edit-dashed-line", "edit-dotted-line"]
                .map(|key| (crate::ui_text::text(key).to_owned(), Icon::stroke("")))
                .into(),
            match initial.dash() {
                DashPattern::Solid => 0,
                DashPattern::Dash => 1,
                _ => 2,
            },
            true,
        )
    };
    let entries = items
        .into_iter()
        .enumerate()
        .map(|(i, (label, icon))| MenuItem {
            id: i as u16 + 1,
            label,
            shortcut: String::new(),
            icon,
            selected: i == current,
            enabled: true,
        })
        .collect();
    let selected = {
        let Some(_guard) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(());
        };
        neuro_mini_ui::set_property_expanded(panel, id, true)?;
        let result = neuro_mini_ui::track_menu(panel, anchor, entries, compact);
        if neuro_mini_ui::is_toolbar(panel) {
            neuro_mini_ui::set_property_expanded(panel, id, false)?;
        }
        result?
    };
    let Some(selected) = selected.map(|i| i as usize - 1) else {
        return Ok(());
    };
    if !valid_target(owner, state, target) {
        return Ok(());
    }
    apply::value(
        &mut state.canvas.borrow_mut(),
        id,
        Some(&selected.to_string()),
    )?;
    Ok(())
}
