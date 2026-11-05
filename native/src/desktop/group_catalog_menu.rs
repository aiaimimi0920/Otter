//! 分组目录提供无成员分组的管理入口；子菜单附加成功后由父菜单统一释放。
use super::*;
use crate::workspace::Group;
pub(in crate::desktop) const RENAME_BASE: usize = 0x5200;
pub(in crate::desktop) const DELETE_BASE: usize = 0x5300;
pub(in crate::desktop) const CLOSE_BASE: usize = 0x5400;
pub(in crate::desktop) const LOCK_BASE: usize = 0x5500;
const HIDE_BASE: usize = 0x5600;
const BASES: [usize; 5] = [RENAME_BASE, DELETE_BASE, CLOSE_BASE, LOCK_BASE, HIDE_BASE];

struct Menu(Option<HMENU>);
impl Drop for Menu {
    fn drop(&mut self) {
        if let Some(menu) = self.0 {
            unsafe {
                let _ = DestroyMenu(menu);
            }
        }
    }
}
pub(super) fn handles(command: usize) -> bool {
    BASES
        .iter()
        .any(|base| (*base..*base + crate::workspace::MAX_UNITS).contains(&command))
}
pub(super) unsafe fn append(
    parent: HMENU,
    groups: &[Group],
    current: Option<&UnitId>,
) -> Result<()> {
    for (index, group) in groups.iter().enumerate() {
        let handle = CreatePopupMenu()?;
        let mut owner = Menu(Some(handle));
        AppendMenuW(
            handle,
            MF_STRING
                | if current == Some(&group.id) {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
            JOIN_BASE + index,
            w!("加入此分组"),
        )?;
        for (base, label) in [
            (RENAME_BASE, w!("重命名…")),
            (
                if group.hidden { SHOW_BASE } else { HIDE_BASE },
                if group.hidden {
                    w!("显示分组")
                } else {
                    w!("隐藏分组")
                },
            ),
            (
                LOCK_BASE,
                if group.locked {
                    w!("解锁分组")
                } else {
                    w!("锁定分组")
                },
            ),
            (DELETE_BASE, w!("删除分组（保留图片）")),
            (CLOSE_BASE, w!("关闭分组的全部窗口")),
        ] {
            AppendMenuW(handle, MF_STRING, base + index, label)?;
        }
        AppendMenuW(
            parent,
            MF_POPUP | MF_STRING,
            handle.0 as usize,
            &HSTRING::from(group.name.replace('&', "&&")),
        )?;
        owner.0 = None; // 父菜单取得所有权；后续任一步失败也由父 owner 递归释放。
    }
    Ok(())
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    execute_with(hwnd, state, command, |name| {
        super::super::shape_dialog::show_group_name(hwnd, name)
    })
}
pub(in crate::desktop) unsafe fn execute_with(
    hwnd: HWND,
    state: &State,
    command: usize,
    rename: impl FnOnce(String) -> Option<String>,
) -> Result<()> {
    let Some(base) = BASES
        .iter()
        .copied()
        .find(|base| (*base..*base + crate::workspace::MAX_UNITS).contains(&command))
    else {
        return Ok(());
    };
    let Some(group) = state.group_catalog.borrow().get(command - base).cloned() else {
        return Ok(());
    };
    let request = match base {
        RENAME_BASE => {
            let value = {
                let Some(_guard) = super::super::file_dialog::Guard::begin(&state.file_dialog)
                else {
                    return Ok(());
                };
                rename(group.name)
            };
            let Some(value) = value else {
                return Ok(());
            };
            if !IsWindow(Some(hwnd)).as_bool()
                || !state
                    .group_catalog
                    .borrow()
                    .iter()
                    .any(|entry| entry.id == group.id)
            {
                return Ok(());
            }
            Request::Rename(group.id, value)
        }
        DELETE_BASE => Request::Delete(group.id),
        CLOSE_BASE => Request::Close(group.id),
        LOCK_BASE => Request::SetLocked(group.id, !group.locked),
        _ => Request::SetHidden(group.id, true),
    };
    queue(hwnd, state, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_ranges_are_bounded_and_do_not_overlap_existing_groups() {
        for base in BASES {
            assert!(handles(base));
            assert!(handles(base + crate::workspace::MAX_UNITS - 1));
            assert!(!handles(base + crate::workspace::MAX_UNITS));
            assert!(super::super::super::group_lock::allows(base));
        }
        assert!(!handles(JOIN_BASE));
        assert!(!handles(SHOW_BASE));
    }
    #[test]
    fn catalog_builds_owned_submenus_with_checked_membership() {
        unsafe {
            let parent = Menu(Some(CreatePopupMenu().unwrap()));
            let group = Group {
                id: UnitId::parse("empty").unwrap(),
                name: "空&组".into(),
                hidden: true,
                locked: true,
            };
            append(
                parent.0.unwrap(),
                std::slice::from_ref(&group),
                Some(&group.id),
            )
            .unwrap();
            assert_eq!(GetMenuItemCount(parent.0), 1);
            let child = GetSubMenu(parent.0.unwrap(), 0);
            assert_eq!(GetMenuItemCount(Some(child)), 6);
            assert_eq!(GetMenuItemID(child, 0), JOIN_BASE as u32);
            assert_eq!(GetMenuItemID(child, 2), SHOW_BASE as u32);
            assert_ne!(
                GetMenuState(child, JOIN_BASE as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
            let edit = super::super::super::context_menu::APPLY_STYLE;
            AppendMenuW(child, MF_STRING, edit, w!("编辑测试")).unwrap();
            let state = State::new(None, false).unwrap();
            state.group_locked.set(true);
            super::super::super::group_lock::restrict_menu(parent.0.unwrap(), &state);
            assert_ne!(
                GetMenuState(child, edit as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
            assert_eq!(
                GetMenuState(child, DELETE_BASE as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                0
            );
        }
    }
}
