//! Live 主动连线的应用事务；共享选择框，确认后重验目录/两端 owner，不在模态中改图。
use super::Application;
use crate::{desktop::link_catalog::Catalog, workspace_id::UnitId};
use std::{cell::Cell, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::Foundation::*,
};
impl Application {
    pub(in crate::desktop::application) unsafe fn process_live_link_requests(&mut self) {
        if self.modals.blocked() {
            return;
        }
        let units: Vec<_> = self
            .live_sessions
            .iter()
            .filter(|session| session.take_link_request())
            .map(|session| session.unit_id().clone())
            .collect();
        for unit in units {
            if let Err(error) = self.create_live_link_with(&unit, |_, hwnd, catalog, source| {
                crate::desktop::link_create::show_outgoing(hwnd, catalog, source)
            }) {
                // 与静态连线一致：过期/关闭来源不再弹出无 owner 的阻塞窗口。
                eprintln!("Live link creation rejected: {error}");
            }
        }
    }
    pub(in crate::desktop::application) fn live_link_owner(&self, unit: &UnitId) -> Option<HWND> {
        if !self.workspace.contains(unit)
            || self.link_owner_blocked(unit)
            || self
                .workspace
                .group_of(unit)
                .is_some_and(|group| group.hidden)
        {
            return None;
        }
        self.live_sessions
            .iter()
            .find(|session| session.unit_id() == unit && session.belongs_to(&self.workspace))
            .and_then(crate::desktop::composition::live::session::Session::group_dialog_owner)
    }
    pub(in crate::desktop::application) fn live_link_busy(&self) -> bool {
        self.save_job.is_some()
            || self.restore_job.is_some()
            || self.live_freeze_job.is_some()
            || self.exit_requested.get()
    }
    pub(in crate::desktop::application) unsafe fn create_live_link_with(
        &mut self,
        unit: &UnitId,
        choose: impl FnOnce(
            &mut Self,
            HWND,
            Rc<Catalog>,
            usize,
        ) -> Result<Option<(usize, usize, usize)>>,
    ) -> Result<bool> {
        if self.modals.blocked() || self.live_link_busy() {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy during Live link dialog",
            ));
        }
        let owner = self
            .live_link_owner(unit)
            .ok_or_else(|| Error::new(E_ABORT, "Live link owner changed"))?;
        let catalog = Catalog::capture(&self.documents, &self.live_sessions, &self.workspace);
        let source = catalog
            .endpoints
            .iter()
            .position(|endpoint| &endpoint.unit == unit)
            .ok_or_else(|| Error::new(E_ABORT, "Live link source missing"))?;
        let flag = Rc::new(Cell::new(false));
        self.modals.register(&flag);
        let guard = crate::desktop::file_dialog::Guard::begin(&flag).unwrap();
        let choice = choose(self, owner, catalog.clone(), source);
        drop(guard);
        let Some((target, output, input)) = choice? else {
            return Ok(false);
        };
        if self.modals.blocked()
            || self.live_link_busy()
            || self.live_link_owner(unit) != Some(owner)
            || !catalog.matches(&self.documents, &self.live_sessions, &self.workspace)
        {
            return Err(Error::new(
                E_ABORT,
                "Live link endpoints changed during dialog",
            ));
        }
        let link = catalog
            .make_link(source, target, output, input)?
            .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid Live link endpoint or port"))?;
        if self.link_owner_blocked(&link.to_unit) || self.link_owner_blocked(&link.from_unit) {
            return Err(Error::new(E_ABORT, "Live link endpoint is busy or locked"));
        }
        self.workspace
            .connect(link)
            .map_err(|error| Error::new(E_INVALIDARG, format!("Live link rejected: {error:?}")))?;
        Ok(true)
    }
}
