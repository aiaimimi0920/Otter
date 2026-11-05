//! 工作区保存入口；窗口回调只排队，应用循环拥有对话框、后台任务与最终提交。
use super::*;
use std::{os::windows::ffi::OsStringExt, path::PathBuf, sync::atomic::AtomicBool};
use windows::Win32::UI::Controls::Dialogs::*;

pub(in crate::desktop) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    request_mode(hwnd, state, true)
}
pub(in crate::desktop) unsafe fn request_existing(hwnd: HWND, state: &State) -> Result<()> {
    request_mode(hwnd, state, false)
}
unsafe fn request_mode(hwnd: HWND, state: &State, save_as: bool) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(
            E_INVALIDARG,
            "Save requester window is no longer valid",
        ));
    }
    let requested = if save_as {
        &state.save_session_requested
    } else {
        &state.save_existing_requested
    };
    if !requested.replace(true) {
        if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
            requested.set(false);
            return Err(error);
        }
    }
    Ok(())
}

impl Application {
    pub(super) unsafe fn pending_save_ui(&mut self) {
        self.pending_workspace_save_with(|app, origin, message, warning| {
            app.save_message(origin, message, warning);
        });
    }

    /// 通知出口可注入，隐藏集成检查仍走正式请求、worker 与文件提交路径。
    pub(super) unsafe fn pending_save_ui_with(
        &mut self,
        mut notify: impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        // 模态回调可以泵窗口消息，但不能在模态期间发布工作区保存结果。
        if self.modals.blocked() {
            return;
        }
        if let Some(origin) = self.save_origin.clone() {
            match self.take_save_candidate() {
                Ok(None) => {}
                result => {
                    self.save_origin = None;
                    let result = result.and_then(|prepared| {
                        let prepared =
                            prepared.ok_or_else(|| Error::new(E_FAIL, "Missing save result"))?;
                        prepared.commit_ready(self, &AtomicBool::new(false))
                    });
                    // 新的保存/另存为意图必须先处理，不能被前一个保存的自动退出越过。
                    if self.documents.iter().any(|doc| {
                        doc.state.save_session_requested.get()
                            || doc.state.save_existing_requested.get()
                            || doc.state.open_session_requested.get()
                    }) {
                        self.exit_after_save = false;
                    }
                    let exiting = self.exit_after_save;
                    self.finish_exit_save(result.is_ok());
                    let opening = match self.finish_open_save(result.is_ok()) {
                        Ok(opening) => opening,
                        Err(error) => {
                            notify(
                                self,
                                &origin,
                                &format!("工作区已保存，但无法继续打开。\n{error}"),
                                true,
                            );
                            false
                        }
                    };
                    match result {
                        Ok(published) => {
                            if !exiting && !opening {
                                self.queue_saved_workflow();
                            }
                            let mut message =
                                format!("工作区已保存：\n{}", published.destination.display());
                            let warning = published.retained_recovery.is_some();
                            if let Some((path, error)) = published.retained_recovery {
                                message.push_str(&format!(
                                    "\n旧恢复文件尚未清理：{}\n{error}",
                                    path.display()
                                ));
                            }
                            if (!exiting && !opening) || warning {
                                notify(self, &origin, &message, warning);
                            }
                        }
                        Err(error) => {
                            notify(self, &origin, &format!("工作区保存失败。\n{error}"), true)
                        }
                    }
                }
            }
        }
        let Some((source, save_as)) = self.take_save_request() else {
            if self.save_job.is_none() {
                self.exit_after_save = false;
                self.open_after_save = None;
            }
            return;
        };
        if self.save_job.is_some() {
            self.exit_after_save = false;
            self.open_after_save = None;
            notify(self, &source, "正在保存工作区，请等待完成。", true);
            return;
        }
        let result = if self.save_requires_destination(save_as) {
            self.choose_session_destination(&source)
        } else {
            self.request_save_existing()
                .map(|()| self.save_origin = Some(source.clone()))
        };
        if let Err(error) = result {
            self.exit_after_save = false;
            self.open_after_save = None;
            notify(self, &source, &format!("无法保存工作区。\n{error}"), true);
        }
        // 路径对话框取消也结束退出意图；不能让下一次手动保存意外退出。
        if self.save_job.is_none() {
            self.exit_after_save = false;
            self.open_after_save = None;
        }
    }

    pub(super) fn save_requires_destination(&self, save_as: bool) -> bool {
        save_as
            || self
                .restored_session
                .as_ref()
                .is_none_or(|record| record.saved_path.is_none())
    }
    fn take_save_request(&self) -> Option<(crate::workspace_id::UnitId, bool)> {
        let mut request = None;
        for document in &self.documents {
            let save_as = document.state.save_session_requested.replace(false);
            let existing = document.state.save_existing_requested.replace(false);
            // 同一批请求中明确另存为优先，不能因合并请求意外改成覆盖保存。
            if save_as || (existing && request.is_none()) {
                request = Some((document.state.unit_id.clone(), save_as));
            }
        }
        request
    }

    unsafe fn choose_session_destination(
        &mut self,
        source: &crate::workspace_id::UnitId,
    ) -> Result<()> {
        let (receipt, data) = self.capture_session_snapshot()?;
        // 此处只需凭证；不要在模态对话框期间额外保留整套编辑器快照。
        drop(data);
        let document = self
            .documents
            .iter()
            .find(|document| &document.state.unit_id == source)
            .ok_or_else(|| Error::new(E_ABORT, "Save requester closed"))?;
        let hwnd = document.window.0;
        let flag = document.state.file_dialog.clone();
        let dialog = super::super::file_dialog::Guard::begin(&flag)
            .ok_or_else(|| Error::new(E_ABORT, "Another file dialog is active"))?;
        let mut name = vec![0u16; 32768];
        let mut options = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: hwnd,
            lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
            nMaxFile: name.len() as u32,
            lpstrFilter: w!("Otter workspace\0*.json\0\0"),
            lpstrDefExt: w!("json"),
            lpstrTitle: w!("工作区另存为（新文件）"),
            Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
            ..Default::default()
        };
        let selected = GetSaveFileNameW(&mut options).as_bool();
        let error = CommDlgExtendedError();
        drop(dialog);
        if !selected {
            return if error == COMMON_DLG_ERRORS(0) {
                Ok(())
            } else {
                Err(Error::new(E_FAIL, format!("Save dialog failed: {error:?}")))
            };
        }
        if !IsWindow(Some(hwnd)).as_bool() || !receipt.matches(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed while choosing save destination",
            ));
        }
        let end = name
            .iter()
            .position(|unit| *unit == 0)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Unterminated save destination"))?;
        let path = PathBuf::from(std::ffi::OsString::from_wide(&name[..end]));
        if path
            .try_exists()
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?
        {
            return Err(Error::new(E_ABORT, "目标文件已存在，请选择新的文件名。"));
        }
        self.request_save_new(path)?;
        self.save_origin = Some(source.clone());
        Ok(())
    }

    pub(super) unsafe fn save_message(
        &self,
        origin: &crate::workspace_id::UnitId,
        message: &str,
        warning: bool,
    ) {
        let owner = self.source_window(origin).or_else(|| {
            self.documents
                .iter()
                .find(|document| document.is_alive())
                .map(|document| document.window.0)
        });
        if let Some(owner) = owner {
            MessageBoxW(
                Some(owner),
                &HSTRING::from(message),
                w!("Otter"),
                MB_OK
                    | if warning {
                        MB_ICONWARNING
                    } else {
                        MB_ICONINFORMATION
                    },
            );
        }
    }
}

/// 隐藏 HWND 通过正式菜单分发排队，不打开可见对话框或写文件。
pub(super) unsafe fn verify_requests(app: &Application) -> Result<()> {
    let document = &app.documents[0];
    let old_lock = document.state.group_locked.replace(true);
    let result =
        super::super::context_menu::execute(document.window.0, &document.state, SAVE_SESSION);
    document.state.group_locked.set(old_lock);
    result?;
    let expected = document.state.unit_id.clone();
    if app.take_save_request() != Some((expected.clone(), false)) {
        return Err(Error::new(
            E_FAIL,
            "Locked group blocked workspace save command",
        ));
    }
    request_existing(document.window.0, &document.state)?;
    request(document.window.0, &document.state)?;
    request_existing(document.window.0, &document.state)?;
    if app.take_save_request() != Some((expected, true)) || app.take_save_request().is_some() {
        return Err(Error::new(
            E_FAIL,
            "Save requests did not coalesce with Save As priority",
        ));
    }
    println!("native-save-menu-smoke: locked-save=true coalesced=true save-as-priority=true");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_requester_does_not_queue_and_modal_request_is_ignored() {
        let state = State::new(None, false).unwrap();
        unsafe {
            assert!(request(HWND::default(), &state).is_err());
            assert!(request_existing(HWND::default(), &state).is_err());
        }
        assert!(!state.save_session_requested.get());
        assert!(!state.save_existing_requested.get());
        state.modals.register(&state.file_dialog);
        state.file_dialog.set(true);
        unsafe {
            request(HWND::default(), &state).unwrap();
            request_existing(HWND::default(), &state).unwrap();
        }
        assert!(!state.save_session_requested.get());
        assert!(!state.save_existing_requested.get());
    }
}
