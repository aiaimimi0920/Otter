//! Live 分组选择由应用循环发布；只接受当前工作区及菜单打开时的完整分组快照。
use super::*;
#[path = "application_live_group_name.rs"]
mod name;
use crate::desktop::composition::live::window::groups::Action;
impl Application {
    pub(in crate::desktop::application) fn sync_live_groups(&self) {
        for session in &self.live_sessions {
            if let Err(error) = session.sync_groups(&self.workspace) {
                eprintln!("Live group menu update failed: {error}");
            }
        }
    }
    pub(in crate::desktop::application) unsafe fn process_live_groups(
        &mut self,
        report: bool,
    ) -> Result<bool> {
        let mut changed = false;
        let requests: Vec<_> = self
            .live_sessions
            .iter()
            .filter_map(|session| {
                session
                    .take_group_request()
                    .map(|request| (session.unit_id().clone(), request))
            })
            .collect();
        for (unit, request) in requests {
            if !self.live_sessions.iter().any(|session| {
                session.unit_id() == &unit
                    && session.is_alive()
                    && session.belongs_to(&self.workspace)
            }) || !request.matches(&self.workspace, &unit)
            {
                continue;
            }
            let result = match &request.action {
                Action::Assign(target) => self
                    .workspace
                    .assign_group(std::slice::from_ref(&unit), target.as_ref())
                    .map(|()| true)
                    .map_err(|error| {
                        Error::new(E_INVALIDARG, format!("Invalid Live group: {error:?}"))
                    }),
                Action::Create | Action::Rename(_) => {
                    self.edit_live_group_with(&unit, &request, |_, hwnd, name| {
                        crate::desktop::shape_dialog::show_group_name(hwnd, name)
                    })
                }
            };
            if let Err(error) = result {
                let message = format!("无法修改 Live 分组：{error}");
                if !report {
                    return Err(Error::new(E_FAIL, message));
                }
                MessageBoxW(
                    None,
                    &HSTRING::from(message),
                    w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            } else {
                changed |= result.unwrap();
            }
        }
        Ok(changed)
    }
}
