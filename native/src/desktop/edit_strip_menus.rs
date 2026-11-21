//! 工具组选项沿原有菜单目标凭证和模态准入交付；不绕过命令可用性与后台任务 owner。
use super::*;
pub(super) unsafe fn show(owner: HWND, state: &State, id: u16) -> Result<usize> {
    show_group(owner, state, id)
}
unsafe fn show_group(owner: HWND, state: &State, id: u16) -> Result<usize> {
    let panel = state.tool_panel.get();
    let Some(available) = Availability::capture(state) else {
        return Ok(0);
    };
    let entries = catalog::choices(id)
        .iter()
        .map(|command| {
            let raw = label(*command);
            neuro_mini_ui::MenuItem {
                id: *command as u16,
                label: raw.split('\t').next().unwrap_or(raw).into(),
                shortcut: String::new(),
                icon: catalog::icon(*command),
                selected: choice(panel, id) == *command,
                enabled: allowed(state, available, *command),
            }
        })
        .collect();
    let mut anchor = RECT::default();
    GetWindowRect(GetDlgItem(Some(panel), id as i32)?, &mut anchor)?;
    let target = crate::desktop::popup_menu::target::MenuTarget::capture(&state.canvas.borrow());
    let result = {
        let Some(_modal) = crate::desktop::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(0);
        };
        neuro_mini_ui::track_menu(panel, anchor, entries, false)?
    };
    Ok(
        if state.lifetime.owns(owner)
            && state.tool_panel.get() == panel
            && target.matches(&state.canvas.borrow())
        {
            result.map_or(0, usize::from)
        } else {
            0
        },
    )
}
pub(super) fn label(command: usize) -> &'static str {
    match command {
        0x1022 => "栅格化所选",
        0x1023 => "栅格化全部",
        _ => crate::desktop::tool_commands::TOOLS
            .iter()
            .find(|entry| entry.id == command)
            .map_or("", |entry| entry.label),
    }
}
