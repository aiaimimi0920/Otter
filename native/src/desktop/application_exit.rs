//! 应用退出确认与异步保存衔接；取消/失败不销毁窗口，不在模态回调里释放 owner。
use super::*;

pub(super) enum Choice {
    Save,
    Discard,
    Cancel,
}

/// 隐藏 HWND 验证决策与完成状态转换；不显示对话框或写用户会话。
pub(super) unsafe fn verify(app: &mut Application) -> Result<()> {
    let saved = app.saved_state.take();
    let count = app.documents.len();
    app.exit_requested.set(true);
    if app.close_requested_with(|_| Choice::Cancel)
        || app.exit_requested.get()
        || app.documents.len() != count
    {
        return Err(Error::new(
            E_FAIL,
            "Cancelled exit changed document ownership",
        ));
    }
    app.exit_requested.set(true);
    if app.close_requested_with(|_| Choice::Save) || !app.exit_after_save {
        return Err(Error::new(E_FAIL, "Save before exit did not queue"));
    }
    for document in &app.documents {
        document.state.save_existing_requested.set(false);
    }
    app.finish_exit_save(false);
    if app.exit_after_save || app.exit_requested.get() {
        return Err(Error::new(E_FAIL, "Failed save retained exit intent"));
    }
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.exit_requested.set(true);
    if app.close_requested_with(|app| {
        app.documents[0].state.canvas.borrow_mut().normal_opacity =
            if opacity == 0.5 { 0.6 } else { 0.5 };
        Choice::Discard
    }) {
        return Err(Error::new(E_FAIL, "Late modal change was discarded"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    app.saved_state = Some(save_state::Stamp::capture(app)?);
    app.documents[0].state.canvas.borrow_mut().normal_opacity =
        if opacity == 0.5 { 0.6 } else { 0.5 };
    app.exit_after_save = true;
    app.finish_exit_save(true);
    if app.exit_requested.get() || app.exit_after_save {
        return Err(Error::new(E_FAIL, "Changed workspace exited after save"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    app.exit_after_save = true;
    app.finish_exit_save(true);
    if !app.exit_requested.replace(false) || app.exit_after_save {
        return Err(Error::new(
            E_FAIL,
            "Unchanged saved workspace did not resume exit",
        ));
    }
    app.saved_state = saved;
    app.exit_saved = None;
    println!("native-exit-save-smoke: cancel-preserved=true save-queued=true failure-preserved=true changed-preserved=true clean-resumed=true");
    Ok(())
}

impl Application {
    pub(super) unsafe fn close_requested(&mut self) -> bool {
        self.close_requested_with(|app| {
            let Some(document) = app.documents.iter().find(|doc| doc.is_alive()) else {
                return Choice::Cancel;
            };
            let flag = document.state.file_dialog.clone();
            let Some(_dialog) = super::super::file_dialog::Guard::begin(&flag) else {
                return Choice::Cancel;
            };
            match MessageBoxW(
                Some(document.window.0),
                w!("工作区有尚未保存到文件的内容。退出前保存到文件吗？\n是：保存到文件后退出\n否：不保存到文件并退出\n取消：继续编辑"),
                w!("退出 Otter"),
                MB_YESNOCANCEL | MB_ICONWARNING | MB_DEFBUTTON3,
            ) {
                IDYES => Choice::Save,
                IDNO => Choice::Discard,
                _ => Choice::Cancel,
            }
        })
    }

    pub(super) unsafe fn close_requested_with(
        &mut self,
        choose: impl FnOnce(&Self) -> Choice,
    ) -> bool {
        self.close_requested_with_notify(choose, |app, message| {
            if let Some(doc) = app.documents.iter().find(|doc| doc.is_alive()) {
                app.save_message(&doc.state.unit_id, message, true);
            }
        })
    }
    pub(super) unsafe fn close_requested_with_notify(
        &mut self,
        choose: impl FnOnce(&Self) -> Choice,
        notify: impl FnOnce(&Self, &str),
    ) -> bool {
        if self.modals.blocked() {
            return false;
        }
        if let Some(ready) = self.finish_checkpoint_exit() {
            return ready && self.close_confirmed();
        }
        if !self.exit_requested.get() {
            return false;
        }
        if self.naming_autosave_blocks_exit()
            || self.workspace_autosave_blocks_exit()
            || self.checkpoint_pending()
        {
            return false;
        }
        self.exit_requested.set(false);
        self.open_after_save = None;
        let approved = self.exit_saved.take();
        if approved
            .as_ref()
            .is_some_and(|intent| !intent.matches(self))
        {
            return false;
        }
        // 正在保存时不插入第二个确认或销毁其来源窗口；完成后必须重新核对状态。
        if self.save_job.is_some() {
            // 已经由保存 UI 拥有的任务可以完成后继续退出；失败仍清除退出意图。
            self.exit_after_save = self.save_origin.is_some();
            return false;
        }
        if self.exit_after_save {
            return false;
        }
        if approved.is_none() && self.save_status() != save_state::Status::Unchanged {
            let Ok(before) = exit_intent::Intent::capture(self) else {
                return false;
            };
            match choose(self) {
                Choice::Cancel => return false,
                Choice::Save => {
                    if let Some(document) = self.documents.iter().find(|doc| doc.is_alive()) {
                        self.exit_after_save =
                            save_ui::request_existing(document.window.0, &document.state).is_ok();
                    }
                    return false;
                }
                // 模态期间可能收到窗口/后台消息，放弃意图不得延伸到后来变化的内容。
                Choice::Discard if !before.matches(self) => return false,
                Choice::Discard => {}
            }
        }
        let result = exit_intent::Intent::capture(self).and_then(|intent| {
            if !intent.matches(self) {
                return Err(Error::new(
                    E_ABORT,
                    "工作区有待完成操作；当前窗口保留，请完成后重试退出。",
                ));
            }
            self.begin_checkpoint_exit(intent)
        });
        match result {
            Ok(true) => false,
            Ok(false) => self.close_confirmed(),
            Err(error) => {
                notify(
                    self,
                    &format!("无法完成退出检查点，当前工作区保留。\n{error}"),
                );
                false
            }
        }
    }
    unsafe fn close_confirmed(&mut self) -> bool {
        self.checkpoint_exiting();
        for document in &self.documents {
            if document.is_alive() {
                SendMessageW(document.window.0, WM_CLOSE, None, None);
            }
        }
        self.collect_closed();
        true
    }

    pub(super) unsafe fn finish_exit_save(&mut self, success: bool) {
        if !std::mem::take(&mut self.exit_after_save)
            || !success
            || !self
                .saved_state
                .as_ref()
                .is_some_and(|stamp| stamp.matches_automatic(self))
        {
            return;
        }
        let Ok(intent) = exit_intent::Intent::capture(self) else {
            return;
        };
        if !intent.matches(self) {
            return;
        }
        // 不在保存结果处理栈中销毁窗口；下一轮消息继续正常退出检查。
        if PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)).is_ok() {
            self.exit_saved = Some(intent);
            self.exit_requested.set(true);
        }
    }
}
