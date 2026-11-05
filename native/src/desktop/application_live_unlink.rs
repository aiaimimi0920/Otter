//! Live 断线复用连线菜单快照；应用模态结束后重验 owner/拓扑，沿唯一图状态提交。
use super::Application;
use crate::{
    desktop::link_menu::{Request, Snapshot},
    workspace_id::UnitId,
};
use std::{cell::Cell, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::Foundation::*,
};

impl Application {
    pub(in crate::desktop::application) unsafe fn process_live_unlink_requests(&mut self) {
        if self.modals.blocked() {
            return;
        }
        let units: Vec<_> = self
            .live_sessions
            .iter()
            .filter(|session| session.take_unlink_request())
            .map(|session| session.unit_id().clone())
            .collect();
        for unit in units {
            if let Err(error) =
                self.unlink_live_with(&unit, |_, owner, snapshot| snapshot.select(owner))
            {
                eprintln!("Live link removal rejected: {error}");
            }
        }
    }
    pub(in crate::desktop::application) unsafe fn unlink_live_with(
        &mut self,
        unit: &UnitId,
        choose: impl FnOnce(&mut Self, HWND, Rc<Snapshot>) -> Result<Option<Request>>,
    ) -> Result<bool> {
        if self.modals.blocked() || self.live_link_busy() {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy during Live unlink menu",
            ));
        }
        let owner = self
            .live_link_owner(unit)
            .ok_or_else(|| Error::new(E_ABORT, "Live unlink owner changed"))?;
        let snapshot = Snapshot::capture(&self.workspace, unit);
        let flag = Rc::new(Cell::new(false));
        self.modals.register(&flag);
        let guard = crate::desktop::file_dialog::Guard::begin(&flag).unwrap();
        let choice = choose(self, owner, snapshot.clone());
        drop(guard);
        let Some(request) = choice? else {
            return Ok(false);
        };
        if self.modals.blocked()
            || self.live_link_busy()
            || self.live_link_owner(unit) != Some(owner)
            || !snapshot.topology.matches(&self.workspace)
            || !self.can_remove_link(unit, &request)
        {
            return Err(Error::new(
                E_ABORT,
                "Live unlink endpoints changed during menu",
            ));
        }
        // 显示缓存、保存脏状态和运行取消继续由既有拓扑凭证消费者收口。
        self.workspace.remove_link(&request.link.id);
        Ok(true)
    }
}
