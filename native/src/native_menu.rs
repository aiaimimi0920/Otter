//! 自绘菜单使用既有 HMENU 构造结果的只读快照，不拥有句柄，也不复制业务命令。
use std::collections::HashSet;
use windows::{
    core::{Error, Result, PWSTR},
    Win32::{Foundation::E_INVALIDARG, UI::WindowsAndMessaging::*},
};
#[path = "native_menu_accessibility.rs"]
pub mod accessibility;
#[path = "native_menu_cascade.rs"]
pub mod cascade;
#[path = "native_menu_model.rs"]
mod model;
#[path = "native_menu_panel.rs"]
pub mod panel;
#[path = "native_menu_scrollbar.rs"]
mod scrollbar;
#[path = "native_menu_uia.rs"]
mod uia;

pub struct Item {
    pub thumbnail: Option<crate::image_thumbnail::Thumbnail>,
    pub command: usize,
    /// 保留 & 助记符和制表符快捷键；绘制层负责解析，不修改业务标签。
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub radio: bool,
    pub default: bool,
    pub separator: bool,
    /// Some(empty) 与普通命令不同：空子菜单不能被误执行。
    pub children: Option<Vec<Item>>,
}

#[derive(Default)]
struct Budget {
    handles: HashSet<usize>,
    items: usize,
    units: usize,
}

fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Unsupported or oversized native menu")
}

/// 仅在构造菜单的 UI 线程调用；调用期间不得修改/销毁树，不泵消息或保留 HWND。
/// 显式拒绝尚未支持的 bitmap/owner-draw，不能无声丢失其业务呈现。
pub fn snapshot(menu: HMENU) -> Result<Vec<Item>> {
    read(menu, 0, &mut Budget::default())
}

fn read(menu: HMENU, depth: usize, budget: &mut Budget) -> Result<Vec<Item>> {
    if depth >= 16 || !budget.handles.insert(menu.0 as usize) {
        return Err(invalid());
    }
    let count = unsafe { GetMenuItemCount(Some(menu)) };
    if count < 0 {
        return Err(Error::from_win32());
    }
    budget.items = budget
        .items
        .checked_add(count as usize)
        .ok_or_else(invalid)?;
    if budget.items > 1024 {
        return Err(invalid());
    }
    let mut items = Vec::with_capacity(count as usize);
    for index in 0..count as u32 {
        let mut info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE
                | MIIM_ID
                | MIIM_STATE
                | MIIM_SUBMENU
                | MIIM_STRING
                | MIIM_BITMAP
                | MIIM_CHECKMARKS,
            ..Default::default()
        };
        unsafe {
            GetMenuItemInfoW(menu, index, true, &mut info)?;
        }
        if info.fType.0 & !(MFT_SEPARATOR.0 | MFT_RADIOCHECK.0) != 0
            // HBMMENU_CALLBACK = -1 也是绘制请求，不能按普通无效 GDI 句柄忽略。
            || !info.hbmpItem.0.is_null()
            || !info.hbmpChecked.0.is_null()
            || !info.hbmpUnchecked.0.is_null()
            || info.cch > 4096
        {
            return Err(invalid());
        }
        budget.units = budget
            .units
            .checked_add(info.cch as usize)
            .ok_or_else(invalid)?;
        if budget.units > 128 * 1024 {
            return Err(invalid());
        }
        let mut label = vec![0u16; info.cch as usize + 1];
        info.dwTypeData = PWSTR(label.as_mut_ptr());
        info.cch = label.len() as u32;
        unsafe {
            GetMenuItemInfoW(menu, index, true, &mut info)?;
        }
        if info.cch as usize >= label.len() {
            return Err(invalid());
        }
        let label = String::from_utf16(&label[..info.cch as usize]).map_err(|_| invalid())?;
        let children = if info.hSubMenu.is_invalid() {
            None
        } else {
            Some(read(info.hSubMenu, depth + 1, budget)?)
        };
        items.push(Item {
            thumbnail: None,
            command: info.wID as usize,
            label,
            enabled: info.fState.0 & (MFS_DISABLED.0 | MFS_GRAYED.0) == 0,
            checked: info.fState.0 & MFS_CHECKED.0 != 0,
            radio: info.fType.0 & MFT_RADIOCHECK.0 != 0,
            default: info.fState.0 & MFS_DEFAULT.0 != 0,
            separator: info.fType.0 & MFT_SEPARATOR.0 != 0,
            children,
        });
    }
    Ok(items)
}

#[cfg(test)]
#[path = "native_menu_tests.rs"]
mod tests;
