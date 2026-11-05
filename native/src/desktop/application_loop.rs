//! Application消息泵：保原分派/保存/准入顺序，紧急观察先于普通业务。
use super::*;
use crate::desktop;

impl Application {
    pub(super) unsafe fn run_loop(
        &mut self,
        startup: Option<desktop::SessionStartup>,
    ) -> Result<()> {
        let initial = &self.documents[0];
        desktop::global_capture::install(initial.window.0, &initial.state);
        for doc in &self.documents {
            doc.observe_priority()?;
            if !doc.state.group_hidden.get() {
                let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
                if !doc.state.canvas.borrow().pinned {
                    let _ = ShowWindow(doc.panel.0, SW_SHOWNOACTIVATE);
                }
            }
        }
        if !initial.state.group_hidden.get() {
            let _ = ShowWindow(initial.window.0, SW_SHOWNORMAL);
        }
        // 初始窗口激活/热键注册先完成，再冻结恢复来源；否则正常启动会使自己的凭证失效。
        if let Some(startup) = startup {
            self.request_restore_input(
                desktop::session_job::Input::File(startup.path),
                startup.root,
                startup.space,
            )?;
        }
        let mut message = MSG::default();
        let mut emergency = crate::emergency_exit::Tracker::default();
        loop {
            let status = self.next_live_message(&mut message)?;
            if status == -1 {
                return Err(Error::from_win32());
            }
            if status == 0 {
                if message.wParam.0 != 0 {
                    return Err(Error::new(E_FAIL, "Native message callback failed"));
                }
                if self.session_end.ended() {
                    self.checkpoint_exiting(); // 系统结束不等于用户删除末Unit，不能写空工作区。
                } else {
                    self.finish_deleted_workspace()?;
                }
                break;
            }
            // 紧急路径不经过保存/模态/业务冻结；原父死恢复owner继续负责Live源恢复。
            if crate::emergency_watchdog::observe_message(
                &mut emergency,
                message.message,
                message.wParam.0,
            ) {
                std::process::exit(0);
            }
            // 记录实际注册者；只有其关闭才向剩余窗口转交，不重启用户主动关闭的热键。
            let hotkey_owner = self
                .documents
                .iter()
                .find(|document| document.state.global_capture.get())
                .map(|document| document.window.0);
            if self.session_end.paused() {
                // 业务宿主另有同步门；这里也不把posted输入交给标准子控件。
                if !desktop::session_end::blocks_input(message.message) {
                    DispatchMessageW(&message);
                }
                continue;
            }
            // 先复制最多4个受owner校验的句柄，键盘翻译可同步派发，不能跨它借用owner。
            let confirmation_key = (WM_KEYFIRST..=WM_KEYLAST).contains(&message.message)
                && self
                    .local_arts
                    .confirmation_handles()
                    .into_iter()
                    .any(|hwnd| desktop::art_local_window::translate_confirmation(hwnd, &message));
            if !self.translate_workspace_chrome(&message)
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
                && !self
                    .documents
                    .iter()
                    .any(|document| desktop::tool_panel::translate(&document.state, &message))
            {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
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
        }
        Ok(())
    }
}
