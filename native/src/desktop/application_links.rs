//! 工作区连线变更的唯一 UI owner；不在弹出菜单的重入消息循环修改拓扑。
use super::Application;
#[path = "application_live_links.rs"]
mod live;
#[path = "application_live_unlink.rs"]
mod unlink;
impl Application {
    fn can_remove_link(
        &self,
        unit: &crate::workspace_id::UnitId,
        request: &crate::desktop::link_menu::Request,
    ) -> bool {
        request.topology.matches(&self.workspace)
            && (request.link.from_unit == *unit || request.link.to_unit == *unit)
            && self.workspace.links().contains(&request.link)
            && !self.link_owner_blocked(&request.link.from_unit)
            && !self.link_owner_blocked(&request.link.to_unit)
    }
    pub(super) fn link_owner_blocked(&self, unit: &crate::workspace_id::UnitId) -> bool {
        if self
            .workspace
            .group_of(unit)
            .is_some_and(|group| group.locked)
        {
            return true;
        }
        if let Some(doc) = self.documents.iter().find(|doc| &doc.state.unit_id == unit) {
            // UI owner 在此短借用中不派发消息；只查询 HWND 存活，不调用窗口过程。
            return !unsafe { doc.is_alive() }
                || doc.state.group_locked.get()
                || doc.state.group_hidden.get()
                || doc.state.pending_adoption.borrow().is_some()
                || doc.state.file_dialog.get();
        }
        !self.live_sessions.iter().any(|session| {
            session.unit_id() == unit
                && session.is_alive()
                && session.belongs_to(&self.workspace)
                && !session.group_input_blocked()
        })
    }
    pub(super) fn sync_link_menus(&self) {
        let existing = self
            .documents
            .first()
            .and_then(|doc| doc.state.link_catalog.borrow().clone());
        let catalog = existing
            .filter(|catalog| {
                catalog.matches(&self.documents, &self.live_sessions, &self.workspace)
            })
            .unwrap_or_else(|| {
                crate::desktop::link_catalog::Catalog::capture(
                    &self.documents,
                    &self.live_sessions,
                    &self.workspace,
                )
            });
        for document in &self.documents {
            *document.state.link_catalog.borrow_mut() = Some(catalog.clone());
            let mut menu = document.state.link_menu.borrow_mut();
            if menu
                .as_ref()
                .is_some_and(|menu| menu.topology.matches(&self.workspace))
            {
                continue;
            }
            *menu = Some(crate::desktop::link_menu::Snapshot::capture(
                &self.workspace,
                &document.state.unit_id,
            ));
        }
    }
    pub(super) fn pending_link_changes(&mut self) {
        if self.modals.blocked() {
            return;
        }
        for document in &self.documents {
            let Some(request) = document.state.link_request.borrow_mut().take() else {
                continue;
            };
            if !document.state.group_locked.get()
                && self.can_remove_link(&document.state.unit_id, &request)
            {
                self.workspace.remove_link(&request.link.id);
            }
        }
        let requests: Vec<_> = self
            .documents
            .iter()
            .filter_map(|document| {
                document
                    .state
                    .link_creation
                    .borrow_mut()
                    .take()
                    .filter(|request| request.link.to_unit == document.state.unit_id)
            })
            .chain(self.chrome.take())
            .collect();
        for request in requests {
            self.apply_link_creation(request);
        }
        // 后续 cancel_requested_executions 按拓扑凭证退休旧执行，不另建取消策略。
        unsafe {
            self.process_live_unlink_requests();
            self.process_live_link_requests();
        }
        self.sync_link_menus();
        unsafe {
            self.process_port_settings();
        }
    }
    pub(super) fn apply_link_creation(&mut self, request: crate::desktop::link_catalog::Creation) {
        if !request
            .catalog
            .matches(&self.documents, &self.live_sessions, &self.workspace)
            || !request.catalog.permits(&request.link)
            || [&request.link.from_unit, &request.link.to_unit]
                .iter()
                .any(|id| self.link_owner_blocked(id))
        {
            return;
        }
        if let Err(error) = self.workspace.connect(request.link) {
            eprintln!("Link creation rejected: {error:?}");
        }
    }
}
