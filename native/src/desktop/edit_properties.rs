//! 二级编辑条的目标凭证、字段提交与工具栏接线；控件/外壳唯一归主题库。
use super::{
    canvas::Canvas, popup_menu::target::MenuTarget, tool_panel_availability::Availability, State,
};
use windows::{
    core::Result,
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};
#[path = "edit_property_apply.rs"]
mod apply;
#[path = "edit_property_color.rs"]
mod color_picker;
#[path = "edit_property_fields.rs"]
mod fields;
#[path = "edit_property_font.rs"]
mod font_picker;
#[path = "edit_property_menus.rs"]
mod menus;
#[cfg(test)]
#[path = "edit_property_tests.rs"]
mod tests;
pub(super) const STROKE: u16 = 1;
pub(super) const FILL: u16 = 2;
pub(super) const WIDTH: u16 = 3;
pub(super) const RADIUS: u16 = 4;
pub(super) const DASH: u16 = 5;
pub(super) const CONSTRAIN: u16 = 6;
pub(super) const RATIO: u16 = 7;
pub(super) const STEP: u16 = 8;
pub(super) const SIDES: u16 = 9;
pub(super) const ARROW: u16 = 10;
pub(super) const ANGLE: u16 = 11;
pub(super) const OPACITY: u16 = 12;
pub(super) const SMOOTH: u16 = 13;
pub(super) const GLOW: u16 = 14;
pub(super) const SIZE: u16 = 15;
pub(super) const SERIAL_RADIUS: u16 = 16;
pub(super) const FONT: u16 = 17;
pub(super) const CELL: u16 = 18;
pub(super) const STRENGTH: u16 = 19;
pub(super) const ONLY_MARKS: u16 = 20;
#[derive(Default)]
pub(super) struct Session {
    target: Option<MenuTarget>,
    context: u64,
}
pub(super) unsafe fn refresh(state: &State) {
    let panel = state.tool_panel.get();
    if !neuro_mini_ui::ready(panel) || super::tool_panel::popup::active(state) {
        return;
    }
    let Ok(c) = state.canvas.try_borrow() else {
        return;
    };
    let Ok(mut session) = state.edit_properties.try_borrow_mut() else {
        return;
    };
    if !session.target.as_ref().is_some_and(|t| t.matches(&c)) {
        session.context = session.context.wrapping_add(1);
        session.target = Some(MenuTarget::capture(&c));
    }
    let fields = fields::fields(&c);
    let context = session.context;
    let enabled = ready_with(
        state,
        &c,
        Availability::capture(state).map(Availability::presentation),
    );
    drop(session);
    drop(c);
    if let Err(e) = neuro_mini_ui::set_properties(panel, context, fields, enabled) {
        eprintln!("native-edit-properties-refresh: {e}");
    }
}
fn ready(state: &State, c: &Canvas) -> bool {
    ready_with(state, c, Availability::capture(state))
}
fn ready_with(state: &State, c: &Canvas, available: Option<Availability>) -> bool {
    !state.session_end.paused()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && !c.blocks_wheel()
        && super::capture_input::exclusive_available(state)
        && available.is_some_and(|a| a.allows(super::context_menu::COLOR))
}
pub(super) unsafe fn execute(owner: HWND, state: &State, id: u16) -> Result<()> {
    let panel = state.tool_panel.get();
    let Some(edit) = neuro_mini_ui::take_property_edit(panel, id) else {
        return Ok(());
    };
    if !state.lifetime.owns(owner) || !IsWindowVisible(panel).as_bool() {
        return Ok(());
    }
    let target = {
        let c = state.canvas.borrow();
        let session = state.edit_properties.borrow();
        if edit.context != session.context
            || !session.target.as_ref().is_some_and(|t| t.matches(&c))
            || !ready(state, &c)
        {
            return Ok(());
        }
        if !fields::fields(&c).iter().any(|f| f.id == id) {
            return Ok(());
        }
        MenuTarget::capture(&c)
    };
    if matches!(id, STROKE | FILL | DASH | FONT) {
        super::tool_panel::popup::preserve(state, || menus::execute(owner, state, id, &target))?;
    } else {
        let mut canvas = state.canvas.borrow_mut();
        apply::value(&mut canvas, id, edit.value.as_deref())?;
        if id == RATIO {
            if let Some(active) = edit.active {
                canvas.tool_profiles.shapes.lock_aspect = active;
            }
        }
    }
    if state.lifetime.owns(owner) {
        if id == STRENGTH {
            super::blur_input::start(owner, state);
        }
        super::hit_region::update(owner, state)?;
        let _ = InvalidateRect(Some(owner), None, false);
        super::tool_panel::refresh(owner);
    }
    Ok(())
}
