//! Live 组名草稿事务：复用名称输入框和应用模态域，确认后原子发布组/成员候选。
use super::*;
use crate::{
    desktop::composition::live::window::groups::{Action, Request},
    workspace::Group,
    workspace_id::UnitId,
};
impl Application {
    pub(in crate::desktop::application) unsafe fn edit_live_group_with(
        &mut self,
        unit: &UnitId,
        request: &Request,
        choose: impl FnOnce(&mut Self, HWND, String) -> Option<String>,
    ) -> Result<bool> {
        if self.modals.blocked()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.live_freeze_job.is_some()
            || self.exit_requested.get()
        {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy during Live group dialog",
            ));
        }
        let owner = self
            .live_group_owner(unit, request)
            .ok_or_else(|| Error::new(E_ABORT, "Live group dialog origin changed"))?;
        let old = request
            .name()
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid Live group name request"))?;
        let flag = std::rc::Rc::new(std::cell::Cell::new(false));
        self.modals.register(&flag);
        let guard = crate::desktop::file_dialog::Guard::begin(&flag).unwrap();
        let value = choose(self, owner, old);
        drop(guard);
        let Some(value) = value else {
            return Ok(false);
        };
        if self.exit_requested.get()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.live_freeze_job.is_some()
            || self.live_group_owner(unit, request) != Some(owner)
        {
            return Err(Error::new(E_ABORT, "Live group changed during name dialog"));
        }
        let mut next = self.workspace.clone();
        match &request.action {
            Action::Create => {
                let id = crate::desktop::document_state::new_unit_id()?;
                next.upsert_group(Group {
                    id: id.clone(),
                    name: value,
                    hidden: false,
                    locked: false,
                })
                .map_err(group_error)?;
                next.assign_group(std::slice::from_ref(unit), Some(&id))
                    .map_err(group_error)?;
            }
            Action::Rename(id) => {
                let mut group = next
                    .groups()
                    .iter()
                    .find(|group| &group.id == id)
                    .cloned()
                    .ok_or_else(|| Error::new(E_ABORT, "Live group no longer exists"))?;
                group.name = value;
                next.upsert_group(group).map_err(group_error)?;
            }
            Action::Assign(_) => return Err(Error::new(E_INVALIDARG, "Not a group name request")),
        }
        self.workspace = next;
        Ok(true)
    }
    fn live_group_owner(&self, unit: &UnitId, request: &Request) -> Option<HWND> {
        if !self.workspace.contains(unit) || !request.matches(&self.workspace, unit) {
            return None;
        }
        self.live_sessions
            .iter()
            .find(|session| session.unit_id() == unit && session.belongs_to(&self.workspace))
            .and_then(Session::group_dialog_owner)
    }
}
fn group_error(error: crate::workspace::Error) -> Error {
    Error::new(
        E_INVALIDARG,
        format!("Invalid Live group name or capacity: {error:?}"),
    )
}
