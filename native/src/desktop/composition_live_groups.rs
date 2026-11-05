//! Live 系统分组菜单：父系统菜单拥有 HMENU，选择只交付快照绑定的单槽请求。
use super::*;
use crate::{
    workspace::{Group, Workspace},
    workspace_id::UnitId,
};
pub(in crate::desktop) const JOIN: usize = 0x3000;
pub(in crate::desktop) const LEAVE: usize = 0x2ff0;
pub(in crate::desktop) const CREATE: usize = 0x2fd0;
pub(in crate::desktop) const RENAME: usize = 0x2fe0;
pub(in crate::desktop) enum Action {
    Assign(Option<UnitId>),
    Create,
    Rename(UnitId),
}
#[derive(Clone, PartialEq)]
struct View {
    groups: Vec<Group>,
    current: Option<UnitId>,
}
#[derive(Default)]
pub(super) struct Control {
    pub(super) menu: Option<HMENU>,
    view: Option<View>,
    pub(super) request: Option<Request>,
}
pub(in crate::desktop) struct Request {
    view: View,
    pub action: Action,
}
impl Request {
    pub fn name(&self) -> Option<String> {
        match &self.action {
            Action::Create => Some(String::new()),
            Action::Rename(id) => self
                .view
                .groups
                .iter()
                .find(|group| &group.id == id)
                .map(|group| group.name.clone()),
            Action::Assign(_) => None,
        }
    }
    pub fn matches(&self, workspace: &Workspace, unit: &UnitId) -> bool {
        self.view.groups == workspace.groups()
            && self.view.current.as_ref() == workspace.group_of(unit).map(|group| &group.id)
    }
}
pub(super) unsafe fn install(hwnd: HWND) -> Result<HMENU> {
    let menu = CreatePopupMenu()?;
    if let Err(error) = AppendMenuW(
        GetSystemMenu(hwnd, false),
        MF_POPUP,
        menu.0 as usize,
        w!("分组"),
    ) {
        let _ = DestroyMenu(menu);
        return Err(error);
    }
    Ok(menu)
}
pub(super) fn handles(command: usize) -> bool {
    [CREATE, RENAME, LEAVE].contains(&command)
        || (JOIN..JOIN + crate::workspace::MAX_UNITS * 16).contains(&command)
}
pub(super) fn queue(state: &State, command: usize) {
    let mut control = state.groups.borrow_mut();
    if control.request.is_some() {
        return;
    }
    let Some(view) = control.view.as_ref() else {
        return;
    };
    let action = if command == CREATE {
        Action::Create
    } else if command == RENAME {
        let Some(id) = view.current.clone() else {
            return;
        };
        Action::Rename(id)
    } else if command == LEAVE {
        if view.current.is_none() {
            return;
        }
        Action::Assign(None)
    } else {
        let Some(group) = view.groups.get((command - JOIN) / 16) else {
            return;
        };
        Action::Assign(Some(group.id.clone()))
    };
    control.request = Some(Request {
        view: view.clone(),
        action,
    });
}
impl LiveWindow {
    pub(in crate::desktop) fn group_dialog_owner(&self) -> Option<HWND> {
        self.is_alive().then_some(self.hwnd)
    }
    pub(in crate::desktop) fn group_input_blocked(&self) -> bool {
        self.state.group_locked.get() || self.state.group_hidden.get()
    }
    pub(in crate::desktop) fn take_group_request(&self) -> Option<Request> {
        self.state.groups.borrow_mut().request.take()
    }
    pub(in crate::desktop) fn sync_groups(&self, workspace: &Workspace) -> Result<()> {
        if !self.is_alive() {
            return Ok(());
        }
        let group = workspace.group_of(&self.unit_id);
        let locked = group.is_some_and(|group| group.locked);
        if self.state.group_locked.replace(locked) != locked && locked {
            unsafe {
                local::drag::cancel(self.hwnd, &self.state);
            }
            self.state.input.borrow_mut().cancel();
        }
        let hidden = group.is_some_and(|group| group.hidden);
        if self.state.group_hidden.replace(hidden) != hidden {
            if hidden {
                unsafe {
                    local::drag::cancel(self.hwnd, &self.state);
                }
                self.state.input.borrow_mut().cancel();
            }
            unsafe {
                if hidden {
                    self.state
                        .group_was_visible
                        .set(IsWindowVisible(self.hwnd).as_bool());
                    let _ = ShowWindow(self.hwnd, SW_HIDE);
                } else if self.state.group_was_visible.replace(false) {
                    let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                }
            }
        }
        let mut control = self.state.groups.borrow_mut();
        if control.view.as_ref().is_some_and(|view| {
            view.groups == workspace.groups()
                && view.current.as_ref() == group.map(|group| &group.id)
        }) {
            return Ok(());
        }
        let view = View {
            groups: workspace.groups().to_vec(),
            current: group.map(|group| group.id.clone()),
        };
        // 更新失败时旧命令也不可执行，避免部分菜单与旧数组错位。
        control.view = None;
        let menu = control
            .menu
            .ok_or_else(|| Error::new(E_FAIL, "Live group menu unavailable"))?;
        unsafe {
            while GetMenuItemCount(Some(menu)) > 0 {
                DeleteMenu(menu, 0, MF_BYPOSITION)?;
            }
            AppendMenuW(menu, MF_STRING, CREATE, w!("创建分组…"))?;
            for (index, group) in view.groups.iter().enumerate() {
                let label = windows::core::HSTRING::from(group.name.replace('&', "&&"));
                AppendMenuW(
                    menu,
                    MF_STRING
                        | if view.current.as_ref() == Some(&group.id) {
                            MF_CHECKED
                        } else {
                            MF_UNCHECKED
                        },
                    JOIN + index * 16,
                    &label,
                )?;
            }
            AppendMenuW(
                menu,
                MF_STRING
                    | if view.current.is_some() {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                LEAVE,
                w!("移出当前分组"),
            )?;
            AppendMenuW(
                menu,
                MF_STRING
                    | if view.current.is_some() {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
                RENAME,
                w!("重命名当前分组…"),
            )?;
        }
        control.view = Some(view);
        Ok(())
    }
}
