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
    let (labels, current): (Vec<String>, usize) = (
        vec!["━  实线".into(), "╌  虚线1".into(), "┄  虚线2".into()],
        match initial.dash() {
            DashPattern::Solid => 0,
            DashPattern::Dash => 1,
            _ => 2,
        },
    );
    let entries = labels
        .iter()
        .enumerate()
        .map(|(i, label)| MenuItem {
            id: i as u16 + 1,
            label: label.clone(),
            shortcut: String::new(),
            icon: Icon::stroke(""),
            selected: i == current,
            enabled: true,
        })
        .collect();
    let selected = {
        let Some(_guard) = super::super::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(());
        };
        neuro_mini_ui::set_property_expanded(panel, DASH, true)?;
        let result = neuro_mini_ui::track_menu(panel, anchor, entries, true);
        if neuro_mini_ui::is_toolbar(panel) {
            neuro_mini_ui::set_property_expanded(panel, DASH, false)?;
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
        DASH,
        Some(&selected.to_string()),
    )?;
    Ok(())
}
