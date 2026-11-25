//! Owned port-settings entry: callbacks queue one Open, then Application's safe scheduling opens drafts and revalidates before commit.
use super::*;
#[path = "port_visibility_dialog.rs"]
mod dialog;
#[path = "workspace_port_visibility.rs"]
pub(super) mod visibility;
#[cfg(test)]
pub(in crate::desktop) const COMMAND: usize = 0x10a0;
pub(in crate::desktop) const LIVE_COMMAND: usize = 0x1f70;
pub(super) struct Open {
    unit: crate::workspace_id::UnitId,
    hwnd: HWND,
    marker: isize,
    toggle: Option<visibility::Toggle>,
}
struct Draft {
    request: Open,
    catalog: Rc<Catalog>,
    model: dialog::Model,
    protected: bool,
}
pub(in crate::desktop) unsafe fn request(binding: &Binding, hwnd: HWND) -> bool {
    let Some(ui) = binding.borrow().upgrade().filter(|ui| !ui.blocked()) else {
        return false;
    };
    if ui.port_settings.borrow().is_some() {
        return false;
    }
    let Some(node) = ui
        .nodes
        .borrow()
        .iter()
        .find(|n| n.hwnd == hwnd.0 as isize)
        .cloned()
    else {
        return false;
    };
    let Some(scene) = ui.scene.borrow().clone() else {
        return false;
    };
    let Some(endpoint) = scene.catalog.endpoints.get(node.endpoint) else {
        return false;
    };
    if !node.interactive
        || !IsWindowVisible(hwnd).as_bool()
        || GetWindowLongPtrW(hwnd, GWLP_USERDATA) != node.marker
    {
        return false;
    }
    if endpoint.inputs.is_empty() && endpoint.outputs.is_empty() {
        return false;
    }
    let open = Open {
        unit: endpoint.unit.clone(),
        hwnd,
        marker: node.marker,
        toggle: None,
    };
    ui.cancel();
    if PostMessageW(Some(scene.wake), WM_NULL, WPARAM(0), LPARAM(0)).is_err() {
        return false;
    }
    *ui.port_settings.borrow_mut() = Some(open);
    true
}
#[cfg(test)]
pub(in crate::desktop) unsafe fn append(menu: HMENU, state: &desktop::State) -> Result<()> {
    if state.chrome.borrow().upgrade().is_some() {
        AppendMenuW(
            menu,
            MF_STRING,
            COMMAND,
            &HSTRING::from(crate::ui_text::text("workspace-port-display")),
        )?;
    }
    Ok(())
}
pub(in crate::desktop) unsafe fn install_live(hwnd: HWND) -> Result<()> {
    AppendMenuW(
        GetSystemMenu(hwnd, false),
        MF_STRING,
        LIVE_COMMAND,
        &HSTRING::from(crate::ui_text::text("workspace-port-display")),
    )
}
impl Application {
    fn port_settings_ready(&self) -> bool {
        !self.modals.blocked()
            && !self.session_end.paused()
            && !self.capture_active.get()
            && self.save_job.is_none()
            && self.restore_job.is_none()
            && !self.exit_requested.get()
    }
    fn port_settings_current(&self, request: &Open) -> bool {
        !self.link_owner_blocked(&request.unit)
            && unsafe {
                IsWindow(Some(request.hwnd)).as_bool()
                    && IsWindowVisible(request.hwnd).as_bool()
                    && !IsIconic(request.hwnd).as_bool()
                    && GetWindowLongPtrW(request.hwnd, GWLP_USERDATA) == request.marker
            }
            && (self
                .documents
                .iter()
                .any(|d| d.state.unit_id == request.unit && d.window.0 == request.hwnd)
                || self.live_sessions.iter().any(|s| {
                    s.unit_id() == &request.unit
                        && s.persistence_idle()
                        && s.group_dialog_owner() == Some(request.hwnd)
                }))
    }
    unsafe fn prepare_port_settings(&self) -> Option<Draft> {
        let request = self.chrome.ui.port_settings.borrow_mut().take();
        let request = request?;
        if !self.port_settings_ready() || !self.port_settings_current(&request) {
            return None;
        }
        let catalog = Catalog::capture(&self.documents, &self.live_sessions, &self.workspace);
        let endpoint = catalog.endpoints.iter().find(|e| e.unit == request.unit)?;
        let model = dialog::Model::new(endpoint, self.workspace.port_visibility(&request.unit));
        if model.is_empty() {
            return None;
        }
        let protected = self
            .documents
            .iter()
            .find(|d| d.state.unit_id == request.unit)
            .is_none_or(|d| d.state.capture_protection_requested.get());
        Some(Draft {
            request,
            catalog,
            model,
            protected,
        })
    }
    fn commit_port_settings(
        &mut self,
        request: &Open,
        catalog: &Catalog,
        visibility: crate::workspace::ports::Visibility,
    ) {
        // Modal pumps may change owners, catalogs, and global gates; cancellation, expiry, or session end cannot publish appearance.
        if self.port_settings_ready()
            && self.port_settings_current(request)
            && catalog.matches(&self.documents, &self.live_sessions, &self.workspace)
        {
            if let Err(error) = self
                .workspace
                .set_port_visibility(&request.unit, visibility)
            {
                eprintln!("native-port-visibility: {error:?}");
            }
        }
    }
    pub(in crate::desktop::application) unsafe fn process_port_settings(&mut self) {
        let toggle = self
            .chrome
            .ui
            .port_settings
            .borrow()
            .as_ref()
            .is_some_and(|r| r.toggle.is_some());
        if toggle {
            let request = self
                .chrome
                .ui
                .port_settings
                .borrow_mut()
                .take()
                .expect("queued visibility intent");
            visibility::commit(self, request);
            return;
        }
        let Some(Draft {
            request,
            catalog,
            model,
            protected,
        }) = self.prepare_port_settings()
        else {
            return;
        };
        let result = {
            let flag = Rc::new(Cell::new(false));
            self.modals.register(&flag);
            let _guard = desktop::file_dialog::Guard::begin(&flag);
            dialog::show(request.hwnd, model, protected)
        };
        match result {
            Ok(Some(visibility)) => self.commit_port_settings(&request, &catalog, visibility),
            Err(error) => eprintln!("native-port-visibility: {error}"),
            _ => (),
        }
    }
}
#[cfg(test)]
#[path = "workspace_port_settings_tests.rs"]
mod tests;
