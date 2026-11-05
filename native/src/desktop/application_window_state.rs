//! 新窗口接入应用共享状态的唯一入口；先登记模态生命周期，再创建 HWND。
use super::*;
impl Application {
    pub(super) fn sync_capture_windows(&self) {
        let mut windows = self.capture_windows.borrow_mut();
        windows.clear();
        windows.extend(
            self.chrome
                .handles()
                .into_iter()
                .filter(|hwnd| !hwnd.is_invalid()),
        );
        windows.extend(
            self.documents
                .iter()
                .flat_map(|document| {
                    [
                        document.window.0,
                        document.panel.0,
                        document.state.parameters_ui.window(),
                    ]
                })
                .filter(|hwnd| !hwnd.is_invalid()),
        );
    }
    pub(super) unsafe fn prepare_window_at(
        &self,
        state: State,
        observe: bool,
        frame: crate::pin_geometry::PinRect,
    ) -> Result<DocumentWindow> {
        let document =
            DocumentWindow::create_at(self.instance, self.bind_window_state(state), frame)?;
        if observe {
            // 可见发布的可失败准备必须仍在事务提交之前，包括 Live freeze。
            crate::desktop::tool_panel::prepare(document.panel.0, &document.state)?;
            document.observe_priority()?;
        }
        Ok(document)
    }
    pub(super) fn source_window(&self, id: &crate::workspace_id::UnitId) -> Option<HWND> {
        if !self.workspace.contains(id) {
            return None;
        }
        self.documents
            .iter()
            .find(|document| document.state.unit_id == *id && unsafe { document.is_alive() })
            .map(|document| document.window.0)
    }
    pub(super) fn bind_window_state(&self, mut state: State) -> State {
        *state.chrome.borrow_mut() = self.chrome.bind();
        state.capture_history = self.capture_history.view.clone();
        state.naming = self.naming.clone();
        state.cache_policy = self.cache_policy.clone();
        state.frozen_library = self.frozen_library.clone();
        state.copy_authority = self.copy_authority.clone();
        state.editable_clipboard = self.editable_clipboard.clone();
        state.naming_autosave = self.naming_autosave.clone();
        state.workspace_autosave = self.workspace_autosave.clone();
        state.catalog = self.catalog.clone();
        state.authorization = self.authorization.clone();
        state.group_catalog = self.group_catalog.clone();
        state.hotkey_enabled = self.hotkey_enabled.clone();
        state.long_hotkey_enabled = self.long_hotkey_enabled.clone();
        state.live_hotkey_enabled = self.live_hotkey_enabled.clone();
        state.exit_requested = self.exit_requested.clone();
        state.session_end = self.session_end.clone();
        state.capture_active = self.capture_active.clone();
        state.capture_windows = self.capture_windows.clone();
        state.modals = self.modals.clone();
        state.modals.register(&state.file_dialog);
        state
    }
    pub(super) unsafe fn prepare_window(
        &self,
        state: State,
        observe: bool,
    ) -> Result<DocumentWindow> {
        let state = self.bind_window_state(state);
        let document = DocumentWindow::create(self.instance, state, false)?;
        if observe {
            crate::desktop::tool_panel::prepare(document.panel.0, &document.state)?;
            document.observe_priority()?;
        }
        Ok(document)
    }
}
