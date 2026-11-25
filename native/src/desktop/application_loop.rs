//! Application message pump: preserve dispatch/save/admission order, with emergency observation before ordinary business work.
use super::*;
use crate::desktop;

impl Application {
    pub(super) unsafe fn run_loop(
        &mut self,
        startup: Option<desktop::SessionStartup>,
    ) -> Result<()> {
        self.enable_local_loom();
        let initial = &self.documents[0];
        desktop::global_capture::install(initial.window.0, &initial.state);
        self.ensure_tray();
        desktop::extension_catalog::request(initial.window.0, &initial.state)?;
        for doc in &self.documents {
            doc.observe_priority()?;
            doc.show_content(SW_SHOWNOACTIVATE);
        }
        initial.show_content(SW_SHOWNORMAL);
        // Finish initial window activation/hotkey registration before freezing restore sources, or normal startup would invalidate its own credentials.
        if let Some(startup) = startup {
            self.request_restore_input(
                desktop::session_job::Input::File(startup.path),
                startup.root,
                startup.space,
            )?;
        }
        let mut message = MSG::default();
        let mut emergency = crate::emergency_exit::Tracker::default();
        let mut keep_hotkeys = self.hotkey_enabled.get();
        loop {
            if self.documents.iter().any(|doc| doc.is_alive()) {
                keep_hotkeys = self.hotkey_enabled.get();
            }
            let status = self.next_live_message(&mut message)?;
            if status == -1 {
                return Err(Error::from_win32());
            }
            if status == 0 {
                if message.wParam.0 != 0 {
                    return Err(Error::new(E_FAIL, "Native message callback failed"));
                }
                if !self.session_end.ended()
                    && !self.exit_committed
                    && !self.documents.iter().any(|doc| doc.is_alive())
                {
                    self.resume_empty_workspace(keep_hotkeys)?;
                    continue;
                }
                if self.session_end.ended() {
                    self.checkpoint_exiting(); // System shutdown is not user deletion of the last Unit; do not write an empty workspace.
                } else {
                    self.finish_deleted_workspace()?;
                }
                break;
            }
            // The emergency path bypasses save/modal/business freezes; the original parent-death recovery owner still restores Live sources.
            if crate::emergency_watchdog::observe_message(
                &mut emergency,
                message.message,
                message.wParam.0,
            ) {
                std::process::exit(0);
            }
            // Record the actual registrant; transfer only when it closes, without restarting hotkeys the user deliberately disabled.
            let hotkey_owner = self
                .documents
                .iter()
                .find(|document| document.state.global_capture.get())
                .map(|document| document.window.0);
            if self.session_end.paused() {
                // Business hosts have a separate synchronous gate; do not forward posted input to standard child controls here either.
                if !desktop::session_end::blocks_input(message.message) {
                    DispatchMessageW(&message);
                }
                continue;
            }
            // Toolbars may retain focus elsewhere; finish in-place text editing before external clicks/saving change the document.
            if self.documents.iter().any(|document| {
                desktop::text_input::inline::before_message(
                    document.window.0,
                    &document.state,
                    &message,
                )
            }) {
                continue;
            }
            // First copy at most 4 owner-validated handles; keyboard translation may dispatch synchronously, so do not borrow the owner across it.
            let confirmation_key = (WM_KEYFIRST..=WM_KEYLAST).contains(&message.message)
                && self
                    .local_arts
                    .confirmation_handles()
                    .into_iter()
                    .any(|hwnd| desktop::art_local_window::translate_confirmation(hwnd, &message));
            let share = self.authorization.borrow().share.clone();
            let share_key = share
                .as_ref()
                .is_some_and(|window| window.translate(&message));
            if !share_key
                && !self
                    .documents
                    .iter()
                    .any(|d| desktop::pin_settings::translate_targets(&d.state, &message))
                && !self.translate_extension_shortcuts(&message)
                && !self.translate_workspace_chrome(&message)
                && !confirmation_key
                && !self
                    .history_window
                    .as_ref()
                    .is_some_and(|window| window.translate(&message))
                && !self.documents.iter().any(|document| {
                    document
                        .parameters
                        .as_ref()
                        .is_some_and(|panel| panel.translate(&message))
                })
                && !self.documents.iter().any(|document| {
                    desktop::extension_notices::translate(&document.state, &message)
                        || desktop::tool_panel::translate(&document.state, &message)
                })
            {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            self.process_context_menu();
            if self.close_requested() {
                continue;
            }
            self.pending_groups(true)?;
            let transfer = hotkey_owner.is_some_and(|owner| {
                !self
                    .documents
                    .iter()
                    .any(|document| document.window.0 == owner && document.is_alive())
            });
            desktop::application_hotkey::process_with(
                &self.documents,
                |hwnd, state, enabled| desktop::global_capture::set(hwnd, state, enabled),
                |source, error| {
                    if IsWindow(Some(source)).as_bool() {
                        MessageBoxW(
                            Some(source),
                            &HSTRING::from(format!("无法切换 Ctrl+1 全局截图快捷键。\n{error}")),
                            w!("Otter"),
                            MB_OK | MB_ICONWARNING,
                        );
                    }
                },
            );
            self.pending_windows(true)?;
            self.pending_save_ui();
            self.pending_open_session();
            self.pending_naming_import();
            self.pending_naming_save();
            self.sync_frozen_menu();
            if transfer
                && !self
                    .documents
                    .iter()
                    .any(|document| document.state.global_capture.get())
            {
                if let Some(document) = self.documents.first() {
                    desktop::global_capture::install(document.window.0, &document.state);
                }
            }
            self.ensure_tray();
        }
        Ok(())
    }

    unsafe fn ensure_tray(&self) {
        self.ensure_tray_with(|doc| desktop::tray::start(doc.window.0, &doc.state));
    }
    pub(super) unsafe fn ensure_tray_with(
        &self,
        install: impl FnOnce(&DocumentWindow) -> Result<()>,
    ) {
        if self
            .documents
            .iter()
            .any(|doc| doc.is_alive() && desktop::tray::is_resident(&doc.state))
        {
            return;
        }
        // The resident icon attaches to a live host; hand it to the new host after closure or full session replacement, rather than creating one per pin.
        if let Some(doc) = self.documents.iter().find(|doc| doc.is_alive()) {
            if let Err(error) = install(doc) {
                eprintln!("native-tray-start: {error}");
                desktop::tray::expose_empty_host(doc.window.0, &doc.state);
            }
        }
    }
}

#[cfg(test)]
#[path = "application_tray_tests.rs"]
mod tray_tests;
