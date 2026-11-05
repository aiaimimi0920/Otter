//! 观看宿主快照；菜单只排队，Application复核附件或在线Art证明后才启动原Session。
use super::{queue, Request, State, View as AuthorizationView};
use crate::{
    desktop::composition::relay::sessions::{ViewerHost, ViewerKey, ViewerState},
    live_relay_viewer_identity::Identity,
};
use std::sync::Arc;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(in crate::desktop) const WATCH: usize = 0x2000;
const STOP_BASE: usize = WATCH + 1;
const MAX_VIEWERS: usize = 4;
const MAX_HOSTS: usize = 8;
const MAX_CATALOG: usize = 512;
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub bindings: Vec<ViewerHost>,
    pub watching: Vec<ViewerState>,
    pub status: String,
}
pub(in crate::desktop) struct Selection {
    pub origin: String,
    pub identity: Identity,
    pub binding: ViewerHost,
}
pub(super) fn handles(command: usize) -> bool {
    (WATCH..STOP_BASE + MAX_VIEWERS).contains(&command)
}
fn available(view: &AuthorizationView) -> bool {
    !view.closed
        && !view.busy
        && !view.catalog_busy
        && view.request.is_none()
        && !view.live.bindings.is_empty()
        && view.catalog.as_ref().is_some_and(|catalog| {
            view.origin.as_deref() == Some(catalog.origin.as_str()) && !catalog.entries.is_empty()
        })
}
fn duplicate(view: &AuthorizationView, origin: &str, session: &str) -> bool {
    view.live
        .watching
        .iter()
        .any(|entry| entry.key.origin() == origin && entry.key.session() == session)
}
pub(in crate::desktop) fn current(view: &AuthorizationView, selected: &Selection) -> bool {
    !view.closed
        && !view.busy
        && !view.catalog_busy
        && view.origin.as_deref() == Some(&selected.origin)
        && view.catalog.as_ref().is_some_and(|catalog| {
            catalog.origin == selected.origin
                && catalog
                    .entries
                    .iter()
                    .any(|entry| entry.identity == selected.identity)
        })
        && view
            .live
            .bindings
            .iter()
            .any(|binding| binding == &selected.binding)
        && !duplicate(view, &selected.origin, selected.identity.session())
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
fn label(value: &str) -> String {
    value
        .chars()
        .take(64)
        .collect::<String>()
        .replace('&', "&&")
}
pub(super) unsafe fn append_menu(menu: HMENU, view: &AuthorizationView) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING
            | if available(view) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        WATCH,
        w!("加入远程观看…"),
    )?;
    if !view.live.status.is_empty() {
        AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            &HSTRING::from(label(&view.live.status)),
        )?;
    }
    for (index, entry) in view.live.watching.iter().take(MAX_VIEWERS).enumerate() {
        let status = if entry.stopping {
            "停止中"
        } else if entry.shown {
            "观看中"
        } else {
            "加入中"
        };
        AppendMenuW(
            menu,
            MF_STRING
                | if entry.stopping || view.closed {
                    MF_GRAYED
                } else {
                    MF_ENABLED
                },
            STOP_BASE + index,
            &HSTRING::from(format!(
                "停止观看：{} · {status}",
                label(entry.key.session())
            )),
        )?;
    }
    Ok(())
}
unsafe fn choices(
    view: &AuthorizationView,
    binding: &ViewerHost,
) -> Result<(Menu, Vec<Selection>)> {
    if !available(view) {
        return Err(Error::new(
            E_ABORT,
            "Live catalog or Surface binding unavailable",
        ));
    }
    let catalog = view.catalog.as_ref().unwrap();
    if catalog.entries.len() > MAX_CATALOG || view.live.bindings.len() > MAX_HOSTS {
        return Err(Error::new(E_ABORT, "Live menu capacity exceeded"));
    }
    if binding.origin() != catalog.origin || !view.live.bindings.contains(binding) {
        return Err(Error::new(E_ABORT, "Surface selection changed"));
    }
    let menu = Menu(CreatePopupMenu()?);
    let mut selections = Vec::new();
    for entry in &catalog.entries {
        if duplicate(view, &catalog.origin, entry.identity.session()) {
            continue;
        }
        selections.push(Selection {
            origin: catalog.origin.clone(),
            identity: entry.identity.clone(),
            binding: binding.clone(),
        });
        AppendMenuW(
            menu.0,
            MF_STRING,
            selections.len(),
            &HSTRING::from(format!(
                "{} · {}",
                label(entry.identity.session()),
                label(entry.identity.source_device())
            )),
        )?;
    }
    Ok((menu, selections))
}
pub(in crate::desktop) unsafe fn select(
    hwnd: HWND,
    state: &State,
    selected: Selection,
) -> Result<()> {
    if !current(&state.authorization.borrow(), &selected) {
        return Err(Error::new(E_ABORT, "Live selection changed"));
    }
    queue(hwnd, state, Request::WatchLive(selected))
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if command == WATCH {
        let mut point = POINT::default();
        GetCursorPos(&mut point)?;
        // 先选附件，再列最多512会话，避免4×512子菜单越过公共1024项快照预算。
        let bindings = state.authorization.borrow().live.bindings.clone();
        if bindings.is_empty() || bindings.len() > MAX_HOSTS {
            return Err(Error::new(E_ABORT, "Surface unavailable"));
        }
        let index = if bindings.len() == 1 {
            0
        } else {
            let menu = Menu(CreatePopupMenu()?);
            for (index, binding) in bindings.iter().enumerate() {
                AppendMenuW(
                    menu.0,
                    MF_STRING,
                    index + 1,
                    &HSTRING::from(format!("{}：{}", binding.kind(), label(binding.label()))),
                )?;
            }
            let command = crate::desktop::popup_menu::track(hwnd, state, menu.0, point, || {
                available(&state.authorization.borrow())
            })?;
            if command == 0 {
                return Ok(());
            }
            command
                .checked_sub(1)
                .filter(|index| *index < bindings.len())
                .ok_or_else(|| Error::new(E_ABORT, "Surface selection expired"))?
        };
        let (menu, mut selections) = choices(&state.authorization.borrow(), &bindings[index])?;
        if selections.is_empty() {
            return Ok(());
        }
        let command = crate::desktop::popup_menu::track(hwnd, state, menu.0, point, || {
            available(&state.authorization.borrow())
        })?;
        if command == 0 {
            return Ok(());
        }
        let index = command
            .checked_sub(1)
            .filter(|index| *index < selections.len())
            .ok_or_else(|| Error::new(E_ABORT, "Live menu selection expired"))?;
        let selected = selections.swap_remove(index);
        select(hwnd, state, selected)
    } else {
        let index = command
            .checked_sub(STOP_BASE)
            .ok_or_else(|| Error::new(E_ABORT, "Invalid live command"))?;
        let key: Arc<ViewerKey> = state
            .authorization
            .borrow()
            .live
            .watching
            .get(index)
            .filter(|entry| !entry.stopping)
            .ok_or_else(|| Error::new(E_ABORT, "Live viewer selection expired"))?
            .key
            .clone();
        queue(hwnd, state, Request::StopLive(key))
    }
}
#[cfg(test)]
#[path = "device_live_viewer_ui_tests.rs"]
mod tests;
