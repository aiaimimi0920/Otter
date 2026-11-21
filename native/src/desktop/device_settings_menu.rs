//! 左侧设置只组合已有授权/投射菜单；弹层结束复核目录身份，不复制请求或网络执行器。
use super::*;
#[path = "device_settings_target.rs"]
mod target;
pub(in crate::desktop) use target::Snapshot;
#[derive(Clone, Copy)]
pub(in crate::desktop) enum Section {
    Loom,
    Receive,
    TrayReceive,
    Flow,
    Policy,
    Devices,
    Groups,
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
pub(in crate::desktop) unsafe fn items(
    section: Section,
    state: &State,
) -> Result<Vec<crate::native_menu::Item>> {
    let menu = Menu(CreatePopupMenu()?);
    let view = state.authorization.borrow();
    match section {
        Section::Loom => append_connection(menu.0, &view)?,
        Section::Devices | Section::Groups => targets::append_menu(menu.0, &view, &state.unit_id)?,
        Section::Policy => policy::append_menu(menu.0, &view)?,
        Section::Receive | Section::TrayReceive => {
            policy::append_menu(menu.0, &view)?;
            projection::append_menu(menu.0, &view)?;
            AppendMenuW(menu.0, MF_SEPARATOR, 0, None)?;
            append_actions(menu.0, &view, &[(DISCOVER, "刷新 Live 投射")])?;
            live::append_direct(menu.0, &view)?;
            live::append_stops(menu.0, &view)?;
        }
        Section::Flow => {
            editing::append_menu(menu.0, state, &view)?;
            if GetMenuItemCount(Some(menu.0)) == 0 {
                AppendMenuW(menu.0, MF_STRING | MF_GRAYED, 0, w!("未建立协作"))?;
            }
        }
    }
    let mut result = crate::native_menu::snapshot(menu.0)?;
    // 投射目标原来挂在全局右键菜单下；附属入口直接呈现同一目录，省去重复一级菜单。
    if matches!(section, Section::Devices | Section::Groups) && result.len() == 1 {
        if let Some(children) = result[0].children.take() {
            result = children;
        }
    }
    if matches!(section, Section::Devices | Section::Groups) {
        result.retain(|item| {
            let group = (sender::groups::BASE
                ..sender::groups::STOP_BASE + crate::projection_delivery::targets::MAX_GROUPS)
                .contains(&item.command);
            let device = (sender::BASE..sender::CANCEL).contains(&item.command)
                || (sender::STOP_BASE
                    ..sender::STOP_BASE
                        + crate::projection_delivery::sender::binding::MAX_PER_UNIT)
                    .contains(&item.command);
            match section {
                Section::Devices => !group,
                Section::Groups => !device,
                _ => true,
            }
        });
    }
    while result.first().is_some_and(|item| item.separator) {
        result.remove(0);
    }
    Ok(result)
}
fn permits(items: &[crate::native_menu::Item], command: usize) -> bool {
    command != 0
        && items.iter().any(|item| {
            item.enabled
                && !item.separator
                && (item.command == command
                    || item
                        .children
                        .as_ref()
                        .is_some_and(|items| permits(items, command)))
        })
}
pub(in crate::desktop) unsafe fn show(
    owner: HWND,
    state: &State,
    section: Section,
    anchor: RECT,
) -> Result<()> {
    // 子栏的真实 EDIT/BUTTON 会激活其顶层；菜单跟随同一 owner 家族的当前前台，
    // 业务提交仍复核原钉图。不能强抢焦点，也不能把外部应用当菜单 owner。
    let surface = GetForegroundWindow();
    if surface.is_invalid()
        || GetAncestor(surface, GA_ROOTOWNER) != GetAncestor(owner, GA_ROOTOWNER)
    {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Settings owner is not foreground",
        ));
    }
    choose_with(owner, state, section, |items| {
        let track = if matches!(section, Section::Policy) {
            crate::native_menu::cascade::track_compact
        } else {
            crate::native_menu::cascade::track
        };
        track(
            surface,
            items,
            POINT {
                x: anchor.left,
                y: anchor.bottom,
            },
        )
    })
}
pub(in crate::desktop) unsafe fn choose_with(
    owner: HWND,
    state: &State,
    section: Section,
    choose: impl FnOnce(Vec<crate::native_menu::Item>) -> Result<Option<usize>>,
) -> Result<()> {
    let snapshot = target::Snapshot::capture(state);
    let tree = items(section, state)?;
    let command = {
        let Some(_modal) = crate::desktop::file_dialog::Guard::begin(&state.file_dialog) else {
            return Ok(());
        };
        choose(tree)?
    };
    let Some(command) = command else {
        return Ok(());
    };
    if state.lifetime.owns(owner)
        && if matches!(section, Section::TrayReceive) {
            !state.modals.blocked() && !state.exit_requested.get() && snapshot.same_inputs(state)
        } else {
            snapshot.matches(state)
        }
        && permits(&items(section, state)?, command)
    {
        // 仍沿原请求单槽、权限、来源和持久化路径；不会在菜单回调里访问远端。
        if matches!(section, Section::TrayReceive) && command == live::WATCH_DIRECT {
            live::choose_direct(owner, state, true)?;
        } else {
            execute(owner, state, command)?;
        }
    }
    Ok(())
}
