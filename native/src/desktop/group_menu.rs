//! 分组菜单只生成有界请求；应用循环拥有工作区修改和成员窗口销毁。
use super::State;
use crate::workspace_id::UnitId;
use windows::{
    core::{w, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const CREATE: usize = 0x1052;
pub(super) const LEAVE: usize = 0x1053;
pub(super) const DELETE: usize = 0x1054;
pub(super) const CLOSE: usize = 0x1055;
const RENAME: usize = 0x1056;
pub(super) const LOCK: usize = 0x1057;
pub(super) const HIDE: usize = 0x1058;
const SHOW_BASE: usize = 0x5100;
pub(super) const JOIN_BASE: usize = 0x5000;
#[path = "group_catalog_menu.rs"]
pub(super) mod catalog;
pub(super) enum Request {
    Create(String),
    Assign(UnitId),
    Leave(UnitId),
    Delete(UnitId),
    Close(UnitId),
    Rename(UnitId, String),
    SetLocked(UnitId, bool),
    SetHidden(UnitId, bool),
}
pub(super) fn handles(command: usize) -> bool {
    (CREATE..=HIDE).contains(&command)
        || catalog::handles(command)
        || (JOIN_BASE..JOIN_BASE + crate::workspace::MAX_UNITS).contains(&command)
        || (SHOW_BASE..SHOW_BASE + crate::workspace::MAX_UNITS).contains(&command)
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let groups = state.group_catalog.borrow().clone();
    let current = state.group_id.borrow().clone();
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(menu, MF_STRING, CREATE, w!("创建分组…"))?;
    catalog::append(menu, &groups, current.as_ref())?;
    for (command, label) in [
        (HIDE, w!("隐藏当前分组（托盘可恢复）")),
        (
            LOCK,
            if state.group_locked.get() {
                w!("解锁当前分组")
            } else {
                w!("锁定当前分组")
            },
        ),
        (RENAME, w!("重命名当前分组…")),
        (LEAVE, w!("移出当前分组")),
        (DELETE, w!("删除当前分组（保留图片）")),
        (CLOSE, w!("关闭当前分组的全部窗口（不移入回收站）")),
    ] {
        AppendMenuW(
            menu,
            MF_STRING
                | if current.is_none() {
                    MF_GRAYED
                } else {
                    MF_ENABLED
                },
            command,
            label,
        )?;
    }
    Ok(())
}
pub(super) unsafe fn queue(hwnd: HWND, state: &State, request: Request) -> Result<()> {
    if state.file_dialog.get() || state.group_request.borrow().is_some() {
        return Ok(());
    }
    *state.group_request.borrow_mut() = Some(request);
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        state.group_request.borrow_mut().take();
        return Err(error);
    }
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if state.file_dialog.get() || state.group_request.borrow().is_some() {
        return Ok(());
    }
    if catalog::handles(command) {
        return catalog::execute(hwnd, state, command);
    }
    let request = match command {
        HIDE => {
            let Some(id) = state.group_id.borrow().clone() else {
                return Ok(());
            };
            Request::SetHidden(id, true)
        }
        command if (SHOW_BASE..SHOW_BASE + crate::workspace::MAX_UNITS).contains(&command) => {
            let Some(id) = state
                .group_catalog
                .borrow()
                .get(command - SHOW_BASE)
                .map(|group| group.id.clone())
            else {
                return Ok(());
            };
            Request::SetHidden(id, false)
        }
        LOCK => {
            let Some(id) = state.group_id.borrow().clone() else {
                return Ok(());
            };
            Request::SetLocked(id, !state.group_locked.get())
        }
        CREATE | RENAME => {
            let current = state.group_id.borrow().clone();
            let value = if command == RENAME {
                let Some(name) = state
                    .group_catalog
                    .borrow()
                    .iter()
                    .find(|group| Some(&group.id) == current.as_ref())
                    .map(|group| group.name.clone())
                else {
                    return Ok(());
                };
                name
            } else {
                String::new()
            };
            let value = {
                let Some(_guard) = super::file_dialog::Guard::begin(&state.file_dialog) else {
                    return Ok(());
                };
                super::shape_dialog::show_group_name(hwnd, value)
            };
            let Some(value) = value else {
                return Ok(());
            };
            if !IsWindow(Some(hwnd)).as_bool() || *state.group_id.borrow() != current {
                return Ok(());
            }
            if command == CREATE {
                Request::Create(value)
            } else {
                Request::Rename(current.unwrap(), value)
            }
        }
        LEAVE | DELETE | CLOSE => {
            let Some(id) = state.group_id.borrow().clone() else {
                return Ok(());
            };
            match command {
                LEAVE => Request::Leave(id),
                DELETE => Request::Delete(id),
                _ => Request::Close(id),
            }
        }
        command if command >= JOIN_BASE => {
            let Some(id) = state
                .group_catalog
                .borrow()
                .get(command - JOIN_BASE)
                .map(|group| group.id.clone())
            else {
                return Ok(());
            };
            Request::Assign(id)
        }
        _ => return Ok(()),
    };
    queue(hwnd, state, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_wakeup_clears_request_and_modal_entry_is_ignored() {
        let state = State::new(None, false).unwrap();
        unsafe {
            assert!(queue(
                HWND(-2isize as *mut _),
                &state,
                Request::Create("组".into())
            )
            .is_err());
            assert!(state.group_request.borrow().is_none());
            state.file_dialog.set(true);
            queue(HWND::default(), &state, Request::Create("组".into())).unwrap();
            assert!(state.group_request.borrow().is_none());
        }
    }
}
