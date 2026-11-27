//! Seam between application snapshots/shortcuts and the sole link commit; UI gestures count toward restore/save/session-end idle gates.
use super::*;
use windows::Win32::{Graphics::Gdi::InvalidateRect, UI::Input::KeyboardAndMouse::*};
impl Application {
    pub(in crate::desktop::application) unsafe fn sync_workspace_chrome(&mut self, visible: bool) {
        for doc in &self.documents {
            *doc.state.chrome.borrow_mut() = self.chrome.bind();
        }
        for session in &self.live_sessions {
            session.bind_workspace_chrome(self.chrome.bind());
        }
        if visible && self.capture_active.get() {
            drawer::cancel(&self.chrome.ui);
            candidate_auxiliary::cancel(&self.chrome.ui);
            self.chrome.ui.redraw_after_capture.set(true);
            self.chrome.ui.keyboard.retire();
            self.chrome.ui.cancel();
            self.chrome.ui.pending.borrow_mut().take();
            return; // Screenshots block interaction only, without hiding existing borders and flashing everything back.
        }
        if !visible
            || self.modals.blocked()
            || self.session_end.paused()
            || self.capture_active.get()
            || self.restore_job.is_some()
            || (self.save_job.is_some() && !self.automatic_save_pending())
        {
            window::hide(&self.chrome.ui);
            return;
        }
        let catalog = self
            .documents
            .first()
            .and_then(|doc| doc.state.link_catalog.borrow().clone())
            .or_else(|| {
                self.chrome
                    .ui
                    .scene
                    .borrow()
                    .as_ref()
                    .map(|s| s.catalog.clone())
            })
            .filter(|catalog| {
                catalog.matches(&self.documents, &self.live_sessions, &self.workspace)
            })
            .unwrap_or_else(|| {
                Catalog::capture(&self.documents, &self.live_sessions, &self.workspace)
            });
        let (scene, nodes) = scene::Scene::capture(self, catalog);
        let auxiliary = candidate_auxiliary::capture(self, &nodes);
        let auxiliary_changed = *self.chrome.ui.auxiliary_counts.borrow() != auxiliary;
        *self.chrome.ui.auxiliary_counts.borrow_mut() = auxiliary;
        let changed = self
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .is_none_or(|old| !Rc::ptr_eq(&old.catalog, &scene.catalog));
        if changed {
            self.chrome.ui.cancel();
            self.chrome.ui.keyboard.retire();
        }
        disclosure::sync(&self.chrome.ui, &scene.catalog, &nodes);
        let geometry_changed = *self.chrome.ui.nodes.borrow() != nodes;
        *self.chrome.ui.nodes.borrow_mut() = nodes;
        selection::reconcile(&self.chrome.ui);
        for doc in &self.documents {
            if !doc.state.click.active() {
                let mut snapshot = doc.state.click.chrome.borrow_mut();
                if let Some(s) = snapshot.as_ref() {
                    let nodes = self.chrome.ui.nodes.borrow();
                    // A hidden/minimized node can reappear under the same
                    // owner. Do not discard its disclosure while it is absent.
                    let Some(n) = nodes.iter().find(|n| {
                        n.hwnd == doc.window.0 .0 as isize
                            && n.marker == (&*doc.state as *const State as isize)
                    }) else {
                        continue;
                    };
                    if !s.restore(&doc.state, &self.chrome.ui, &scene, n) || s.drawer.is_none() {
                        snapshot.take();
                    }
                }
            }
        }
        *self.chrome.ui.scene.borrow_mut() = Some(scene);
        keyboard::reconcile(&self.chrome.ui);
        *self.chrome.ui.gates.borrow_mut() = Some(scene::Gates {
            modals: self.modals.clone(),
            session: self.session_end.clone(),
            capture: self.capture_active.clone(),
            exit: self.exit_requested.clone(),
        });
        if self.chrome.windows.is_none() && !self.chrome.ui.nodes.borrow().is_empty() {
            match window::Windows::new(self.chrome.ui.clone()) {
                Ok(windows) => self.chrome.windows = Some(windows),
                Err(error) => {
                    self.chrome.ui.fail(error);
                    self.report_workspace_chrome_failure();
                    return;
                }
            }
        }
        let resume_capture = self.chrome.ui.redraw_after_capture.replace(false);
        if changed
            || geometry_changed
            || auxiliary_changed
            || resume_capture
            || self
                .chrome
                .windows
                .as_ref()
                .is_some_and(window::Windows::language_changed)
            || !IsWindowVisible(self.chrome.ui.drawing.get()).as_bool()
        {
            if let Err(error) = self
                .chrome
                .windows
                .as_mut()
                .map_or(Ok(()), |windows| windows.sync())
            {
                window::hide(&self.chrome.ui);
                self.chrome.ui.fail(error);
            }
            self.sync_capture_windows();
        }
        if let Err(error) = parameter_rail::sync(self) {
            self.chrome.ui.fail(error);
        }
        if let Err(error) = drawer::sync(self) {
            self.chrome.ui.fail(error);
        }
        self.sync_capture_windows();
        self.report_workspace_chrome_failure();
    }
    unsafe fn report_workspace_chrome_failure(&self) {
        let Some(error) = self.chrome.ui.failure.borrow_mut().take() else {
            return;
        };
        self.chrome.ui.reported.set(Some(error.code()));
        window::hide(&self.chrome.ui);
        let owner = self
            .documents
            .iter()
            .find(|doc| doc.is_alive())
            .map(|doc| doc.window.0)
            .or_else(|| {
                self.live_sessions
                    .iter()
                    .find_map(|s| s.group_dialog_owner())
            });
        if let Some(hwnd) = owner {
            let flag = Rc::new(Cell::new(false));
            self.modals.register(&flag);
            let _guard = desktop::file_dialog::Guard::begin(&flag);
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(crate::ui_text::format(
                    "workspace-chrome-unavailable",
                    &[("error", error.to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
    pub(in crate::desktop::application) unsafe fn translate_workspace_chrome(
        &self,
        message: &MSG,
    ) -> bool {
        if drawer::translate(&self.chrome.ui, message) {
            return true;
        }
        if keyboard::translate(&self.chrome.ui, message) {
            return true;
        }
        if message.message != WM_KEYDOWN || self.chrome.ui.blocked() {
            return false;
        }
        let ours = self.documents.iter().any(|doc| {
            doc.is_alive()
                && !doc.state.group_hidden.get()
                && !doc.state.group_locked.get()
                && (message.hwnd == doc.window.0
                    || message.hwnd == doc.panel.0
                    || IsChild(doc.panel.0, message.hwnd).as_bool()
                    || doc
                        .parameters
                        .as_ref()
                        .is_some_and(|panel| panel.chrome_key(message)))
        }) || self
            .live_sessions
            .iter()
            .any(|session| session.group_dialog_owner() == Some(message.hwnd));
        let input = self
            .chrome
            .ui
            .input_windows
            .borrow()
            .iter()
            .any(|(hwnd, _, _)| *hwnd == message.hwnd);
        if message.wParam.0 == VK_ESCAPE.0 as usize
            && (ours || input)
            && self.chrome.ui.press.borrow().is_some()
        {
            self.chrome.ui.cancel();
            self.chrome.ui.repaint();
            return true;
        }
        if (!ours && !input)
            || !chord(
                message.wParam.0,
                GetKeyState(VK_CONTROL.0 as i32) < 0,
                GetKeyState(VK_SHIFT.0 as i32) < 0,
                GetKeyState(VK_MENU.0 as i32) < 0,
                GetKeyState(VK_LWIN.0 as i32) < 0 || GetKeyState(VK_RWIN.0 as i32) < 0,
            )
        {
            return false;
        }
        if message.lParam.0 & (1 << 30) == 0 {
            self.chrome.ui.cancel();
            self.chrome.ui.reported.set(None);
            self.chrome.ui.mode.set(self.chrome.ui.mode.get().next());
            if let Some(windows) = &self.chrome.windows {
                if let Err(error) = windows.refresh() {
                    window::hide(&self.chrome.ui);
                    self.chrome.ui.fail(error);
                }
            }
            for doc in &self.documents {
                let _ = InvalidateRect(Some(doc.window.0), None, false);
            }
            for session in &self.live_sessions {
                if let Some(hwnd) = session.group_dialog_owner() {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
            }
        }
        true
    }
}
fn chord(key: usize, control: bool, shift: bool, alt: bool, win: bool) -> bool {
    key == 0x34 && control && shift && !alt && !win
}
