//! A clicked device is bound to the exact local directory and source authorization.
use crate::{
    device_authorization_lease::Lease,
    projection_delivery::targets::{Directory, Route},
};
use std::sync::Arc;
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{HWND, POINT},
        UI::WindowsAndMessaging::*,
    },
};

struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub(super) fn choose(
    hwnd: HWND,
    lease: &Lease,
    directory: &Arc<Directory>,
) -> Result<Option<String>, String> {
    let targets: Vec<_> = directory
        .targets()
        .iter()
        .filter(|target| {
            matches!(target.route(), Route::SharedLoom) && target.id() != lease.device_id()
        })
        .collect();
    if targets.is_empty() {
        return Err("无可用的同一 Loom 设备，请刷新投送目标".into());
    }
    unsafe {
        let menu = Menu(CreatePopupMenu().map_err(|e| e.to_string())?);
        for (index, target) in targets.iter().enumerate() {
            let name = target
                .name()
                .chars()
                .take(48)
                .collect::<String>()
                .replace('&', "&&");
            // Always show the unique ID too; duplicate friendly names are not destinations.
            AppendMenuW(
                menu.0,
                MF_STRING,
                index + 1,
                &HSTRING::from(format!("{name} · {}", target.id())),
            )
            .map_err(|e| e.to_string())?;
        }
        let mut point = POINT::default();
        GetCursorPos(&mut point).map_err(|e| e.to_string())?;
        let selected = crate::native_menu::cascade::track(
            hwnd,
            crate::native_menu::snapshot(menu.0).map_err(|e| e.to_string())?,
            point,
        )
        .map_err(|e| e.to_string())?;
        Ok(selected
            .and_then(|id| id.checked_sub(1))
            .and_then(|index| targets.get(index))
            .map(|target| target.id().to_owned()))
    }
}
