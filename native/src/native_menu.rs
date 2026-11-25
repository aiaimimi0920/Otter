//! Custom-drawn menus use read-only snapshots of existing HMENU construction results, without owning handles or copying business commands.
use std::collections::HashSet;
use windows::{
    core::{Error, Result, PWSTR},
    Win32::{Foundation::E_INVALIDARG, UI::WindowsAndMessaging::*},
};
#[path = "native_menu_accessibility.rs"]
pub mod accessibility;
#[path = "native_menu_cascade.rs"]
pub mod cascade;
#[path = "native_menu_layout.rs"]
mod layout;
#[path = "native_menu_model.rs"]
mod model;
#[path = "native_menu_panel.rs"]
pub mod panel;
#[path = "native_menu_scrollbar.rs"]
mod scrollbar;
#[path = "native_menu_uia.rs"]
mod uia;

pub struct Item {
    pub delete: Option<usize>,
    pub preview: Option<panel::Preview>,
    pub thumbnail: Option<crate::image_thumbnail::Thumbnail>,
    pub command: usize,
    /// Preserve & mnemonics and tab-separated shortcuts; drawing parses them without changing business labels.
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub radio: bool,
    pub default: bool,
    pub separator: bool,
    /// Some(empty) differs from ordinary commands: empty submenus must not execute accidentally.
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

/// Call only on the menu-building UI thread; do not mutate/destroy the tree, pump messages, or retain HWND during the call.
/// Explicitly reject unsupported bitmap/owner-draw items rather than silently losing their business presentation.
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
            // HBMMENU_CALLBACK = -1 also requests drawing; do not ignore it as an ordinary invalid GDI handle.
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
            delete: None,
            preview: None,
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "native_menu_tests.rs"]
mod tests;
