//! 原生连线菜单只提交身份绑定请求；拓扑修改由应用线程统一处理。
use super::State;
use crate::{
    workspace::{Link, TopologyStamp, Workspace},
    workspace_id::UnitId,
};
use std::rc::Rc;
use windows::{
    core::{w, Result, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const FIRST: usize = 0x8000;
pub(super) struct Snapshot {
    pub topology: TopologyStamp,
    links: Vec<Link>,
}
pub(super) struct Request {
    pub topology: TopologyStamp,
    pub link: Link,
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
impl Snapshot {
    pub fn capture(workspace: &Workspace, unit: &UnitId) -> Rc<Self> {
        Rc::new(Self {
            topology: workspace.topology_stamp(),
            links: workspace
                .links()
                .iter()
                .filter(|link| link.from_unit == *unit || link.to_unit == *unit)
                .cloned()
                .collect(),
        })
    }
    fn handles(&self, command: usize) -> bool {
        command
            .checked_sub(FIRST)
            .is_some_and(|index| index < self.links.len())
    }
    pub unsafe fn queue(
        self: &Rc<Self>,
        hwnd: HWND,
        state: &State,
        command: usize,
    ) -> Result<bool> {
        if !IsWindow(Some(hwnd)).as_bool()
            || state.file_dialog.get()
            || state.group_locked.get()
            || state.link_request.borrow().is_some()
            || state.link_creation.borrow().is_some()
            || !state
                .link_menu
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, self))
        {
            return Ok(false);
        }
        let Some(request) = self.request(command) else {
            return Ok(false);
        };
        *state.link_request.borrow_mut() = Some(request);
        if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
            state.link_request.borrow_mut().take();
            return Err(error);
        }
        Ok(true)
    }
    pub fn request(&self, command: usize) -> Option<Request> {
        let link = self.links.get(command.checked_sub(FIRST)?)?;
        Some(Request {
            topology: self.topology.clone(),
            link: link.clone(),
        })
    }
    /// 返回打开时快照的选择，不在可重入菜单循环中更改拓扑；应用负责复核并提交。
    pub unsafe fn select(&self, owner: HWND) -> Result<Option<Request>> {
        if self.links.is_empty() {
            return Ok(None);
        }
        let menu = Menu(CreatePopupMenu()?);
        self.append(menu.0, false)?;
        let mut point = POINT::default();
        GetCursorPos(&mut point)?;
        let _ = SetForegroundWindow(owner);
        let command = TrackPopupMenu(
            menu.0,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            owner,
            None,
        )
        .0 as usize;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        Ok(self.request(command))
    }
    unsafe fn append(&self, parent: HMENU, locked: bool) -> Result<()> {
        if self.links.is_empty() {
            return Ok(());
        }
        let menu = Menu(CreatePopupMenu()?);
        // 每页最多 64 行，命令仍索引打开时快照，不能按最新列表解释旧命令。
        for (page, links) in self.links.chunks(64).enumerate() {
            let child = Menu(CreatePopupMenu()?);
            for (offset, link) in links.iter().enumerate() {
                let label: String = format!(
                    "{}:{} → {}:{}",
                    link.from_unit, link.from_port, link.to_unit, link.to_port
                )
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .take(160)
                .collect();
                AppendMenuW(
                    child.0,
                    MF_STRING | if locked { MF_GRAYED } else { MF_ENABLED },
                    FIRST + page * 64 + offset,
                    &HSTRING::from(label.replace('&', "&&")),
                )?;
            }
            AppendMenuW(
                menu.0,
                MF_POPUP,
                child.0 .0 as usize,
                &HSTRING::from(format!(
                    "连线 {}–{}",
                    page * 64 + 1,
                    page * 64 + links.len()
                )),
            )?;
            std::mem::forget(child);
        }
        AppendMenuW(parent, MF_POPUP, menu.0 .0 as usize, w!("断开连线"))?;
        std::mem::forget(menu);
        Ok(())
    }
}
pub(super) unsafe fn append(parent: HMENU, state: &State) -> Result<Option<Rc<Snapshot>>> {
    let snapshot = state.link_menu.borrow().clone();
    if let Some(snapshot) = &snapshot {
        snapshot.append(parent, state.group_locked.get())?;
    }
    Ok(snapshot)
}
pub(super) unsafe fn apply(
    snapshot: Option<Rc<Snapshot>>,
    hwnd: HWND,
    state: &State,
    command: usize,
) -> Result<bool> {
    let Some(snapshot) = snapshot.filter(|snapshot| snapshot.handles(command)) else {
        return Ok(false);
    };
    snapshot.queue(hwnd, state, command)?;
    Ok(true)
}

#[cfg(test)]
#[path = "link_menu_tests.rs"]
mod tests;
