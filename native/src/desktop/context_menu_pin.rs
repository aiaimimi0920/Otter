//! Pins retain baseline menus; editing/debug commands stay on toolbars/shortcuts rather than crowding context menus.
use super::*;
pub(super) unsafe fn build(state: &State) -> Result<Menu> {
    let menu = Menu(CreatePopupMenu()?);
    let available =
        state.resolved_display.borrow().is_some() || state.canvas.borrow().image.is_some();
    label(
        menu.0,
        EXPORT,
        "保存",
        MF_STRING | if available { MF_ENABLED } else { MF_GRAYED },
    )?;
    super::super::reference_ui::append_menu(menu.0, state)?;
    Ok(menu)
}
