//! 分组请求在应用循环处理；不跨窗口关闭消息持有工作区或 State 的可变借用。
use super::*;
use crate::desktop::group_menu::Request;
use crate::{
    workspace::{Error as ModelError, Group},
    workspace_id::UnitId,
};
fn model(error: ModelError) -> Error {
    Error::new(E_INVALIDARG, format!("无法修改分组：{error:?}"))
}
impl Application {
    pub(super) unsafe fn sync_groups(&self) {
        *self.group_catalog.borrow_mut() = self.workspace.groups().to_vec();
        for document in &self.documents {
            *document.state.group_id.borrow_mut() = self
                .workspace
                .group_of(&document.state.unit_id)
                .map(|group| group.id.clone());
            let hidden = self
                .workspace
                .group_of(&document.state.unit_id)
                .is_some_and(|group| group.hidden);
            if document.state.group_hidden.replace(hidden) && !hidden {
                super::super::tray::restore_window(document.window.0, &document.state);
            }
            let changed = document.state.group_locked.replace(
                self.workspace
                    .group_of(&document.state.unit_id)
                    .is_some_and(|group| group.locked),
            ) != document.state.group_locked.get();
            if changed {
                super::super::tool_panel::refresh(document.window.0);
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(
                    Some(document.window.0),
                    None,
                    false,
                );
            }
        }
        self.sync_live_groups();
    }
    pub(super) unsafe fn pending_groups(&mut self, report: bool) -> Result<()> {
        if self.modals.blocked() {
            return Ok(());
        }
        self.collect_closed();
        let requests: Vec<_> = self
            .documents
            .iter()
            .filter_map(|document| {
                document
                    .state
                    .group_request
                    .borrow_mut()
                    .take()
                    .map(|request| (document.state.unit_id.clone(), request))
            })
            .collect();
        for (source, request) in requests {
            if self.source_window(&source).is_none() {
                continue;
            }
            let result = self.apply_group(&source, request);
            self.sync_groups();
            if let Err(error) = result {
                if !report {
                    return Err(error);
                }
                if let Some(hwnd) = self.source_window(&source) {
                    MessageBoxW(
                        Some(hwnd),
                        &HSTRING::from(format!("{error}")),
                        w!("Otter"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
        if self.process_live_groups(report)? {
            self.sync_groups();
        } else {
            self.sync_live_groups();
        }
        Ok(())
    }
    unsafe fn apply_group(&mut self, source: &UnitId, request: Request) -> Result<()> {
        match request {
            Request::Create(name) => {
                let id = super::super::document_state::new_unit_id()?;
                self.workspace
                    .upsert_group(Group {
                        id: id.clone(),
                        name,
                        hidden: false,
                        locked: false,
                    })
                    .map_err(model)?;
                if let Err(error) = self
                    .workspace
                    .assign_group(std::slice::from_ref(source), Some(&id))
                {
                    self.workspace.remove_group(&id);
                    return Err(model(error));
                }
            }
            Request::Assign(id) => {
                if self
                    .workspace
                    .groups()
                    .iter()
                    .any(|group| group.id == id && group.locked)
                {
                    self.prepare_group_lock(std::slice::from_ref(source))?;
                }
                let document = self
                    .documents
                    .iter()
                    .find(|document| document.state.unit_id == *source)
                    .unwrap();
                let was_hidden = super::super::tray::is_hidden(&document.state);
                let hidden = self
                    .workspace
                    .groups()
                    .iter()
                    .any(|group| group.id == id && group.hidden);
                if hidden {
                    super::super::tray::hide(document.window.0, &document.state)?;
                }
                let result = self
                    .workspace
                    .assign_group(std::slice::from_ref(source), Some(&id))
                    .map_err(model);
                if result.is_err() && hidden && !was_hidden {
                    super::super::tray::restore_window(document.window.0, &document.state);
                }
                result?;
            }
            Request::SetHidden(id, hidden) => self.set_group_hidden(&id, hidden)?,
            Request::SetLocked(id, locked) => {
                let mut group = self
                    .workspace
                    .groups()
                    .iter()
                    .find(|group| group.id == id)
                    .cloned()
                    .ok_or_else(|| model(ModelError::MissingGroup))?;
                if locked {
                    let members: Vec<_> = self
                        .workspace
                        .units()
                        .iter()
                        .filter(|unit| {
                            self.workspace
                                .group_of(unit)
                                .is_some_and(|group| group.id == id)
                        })
                        .cloned()
                        .collect();
                    self.prepare_group_lock(&members)?;
                }
                group.locked = locked;
                self.workspace.upsert_group(group).map_err(model)?;
            }
            Request::Leave(id) => {
                if self
                    .workspace
                    .group_of(source)
                    .is_some_and(|group| group.id == id)
                {
                    self.workspace
                        .assign_group(std::slice::from_ref(source), None)
                        .map_err(model)?;
                }
            }
            Request::Delete(id) => {
                self.workspace.remove_group(&id);
            }
            Request::Rename(id, name) => {
                let mut group = self
                    .workspace
                    .groups()
                    .iter()
                    .find(|group| group.id == id)
                    .cloned()
                    .ok_or_else(|| model(ModelError::MissingGroup))?;
                group.name = name;
                self.workspace.upsert_group(group).map_err(model)?;
            }
            Request::Close(id) => {
                let members: Vec<_> = self
                    .workspace
                    .units()
                    .iter()
                    .filter(|unit| {
                        self.workspace
                            .group_of(unit)
                            .is_some_and(|group| group.id == id)
                    })
                    .cloned()
                    .collect();
                // 与现有静态批量关闭一致：此明确命令不回收，避免多窗口异步回收半批发布。
                for session in &mut self.live_sessions {
                    if members.contains(session.unit_id()) && session.belongs_to(&self.workspace) {
                        session.stop();
                    }
                }
                self.retain_workspace_owners();
                for member in &members {
                    if let Some(hwnd) = self.source_window(member) {
                        SendMessageW(hwnd, WM_CLOSE, None, None);
                    }
                }
                self.collect_closed();
                // 若未来宿主支持拒绝关闭，保留尚存成员和组，不提前解除关联。
                if members.iter().any(|member| self.workspace.contains(member)) {
                    return Err(Error::new(E_ABORT, "仍有分组窗口未关闭。"));
                }
                self.workspace.remove_group(&id);
            }
        }
        Ok(())
    }
    unsafe fn prepare_group_lock(&self, members: &[UnitId]) -> Result<()> {
        if self
            .documents
            .iter()
            .filter(|document| members.contains(&document.state.unit_id))
            .any(|document| super::super::group_lock::editing_busy(&document.state))
        {
            return Err(Error::new(
                E_ABORT,
                "组内图片正在处理，请完成或取消后再锁定。",
            ));
        }
        for document in self
            .documents
            .iter()
            .filter(|document| members.contains(&document.state.unit_id))
        {
            super::super::menu_transition::prepare(document.window.0, &document.state)?;
        }
        Ok(())
    }
}
