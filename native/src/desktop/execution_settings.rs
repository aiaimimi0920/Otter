//! 节点执行策略的原生勾选菜单；策略修改只退休旧配置，不自动发起执行。
use super::State;
use crate::graph_execution_config::Config;
use windows::{
    core::{w, Result},
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
};
pub(super) const FIRST: usize = 0x1085;
#[cfg(test)]
#[path = "execution_settings_tests.rs"]
mod tests;
pub(super) fn handles(command: usize) -> bool {
    (FIRST..FIRST + 4).contains(&command)
}
fn flags(config: Config) -> [bool; 4] {
    [
        config.upstream_driven,
        config.param_driven,
        config.listen_upstream,
        config.notify_downstream,
    ]
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let selected = state.selected_art.borrow();
    let Some(selected) = selected.as_ref() else {
        return Ok(());
    };
    for (index, (label, checked)) in [
        w!("上游变化触发执行"),
        w!("参数变化触发执行"),
        w!("接收上游执行通知"),
        w!("完成后通知下游"),
    ]
    .into_iter()
    .zip(flags(selected.execution))
    .enumerate()
    {
        AppendMenuW(
            menu,
            MF_STRING
                | if checked { MF_CHECKED } else { MF_UNCHECKED }
                | if state.group_locked.get() {
                    MF_GRAYED
                } else {
                    MF_ENABLED
                },
            FIRST + index,
            label,
        )?;
    }
    Ok(())
}
pub(super) unsafe fn toggle(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if !handles(command) || !IsWindow(Some(hwnd)).as_bool() || state.group_locked.get() {
        return Ok(());
    }
    let mut selected = state.selected_art.borrow_mut();
    let Some(selected) = selected.as_mut() else {
        return Ok(());
    };
    let mut config = selected.execution;
    match command - FIRST {
        0 => config.upstream_driven = !config.upstream_driven,
        1 => config.param_driven = !config.param_driven,
        2 => config.listen_upstream = !config.listen_upstream,
        3 => config.notify_downstream = !config.notify_downstream,
        _ => return Ok(()),
    }
    super::execution_cancel::request(hwnd, state)?;
    selected.execution = config;
    Ok(())
}
