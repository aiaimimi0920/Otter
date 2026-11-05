//! 显式目录授权及保存后同步入口；授权随当前 Record 生命周期，不从图片路径扩权。
#[path = "application_workflow_ui_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
use super::*;
pub(in crate::desktop) const ENABLE: usize = 0x10a0;
pub(in crate::desktop) const DISABLE: usize = 0x10a1;
pub(in crate::desktop::application) struct Settings {
    root: Arc<Root>,
    enabled: bool,
    pending: Option<Source>,
    interactive: bool,
}
pub(in crate::desktop) fn handles(command: usize) -> bool {
    matches!(command, ENABLE | DISABLE)
}
pub(in crate::desktop) unsafe fn request(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    if state.modals.blocked() {
        return Ok(());
    }
    if !handles(command) || !IsWindow(Some(hwnd)).as_bool() {
        return Err(Error::new(E_INVALIDARG, "Invalid workflow requester"));
    }
    let previous = state
        .workflow_sync_requested
        .replace(Some(command == ENABLE));
    if let Err(error) = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)) {
        state.workflow_sync_requested.set(previous);
        return Err(error);
    }
    Ok(())
}
pub(in crate::desktop) unsafe fn append_menu(menu: HMENU) -> Result<()> {
    AppendMenuW(menu, MF_STRING, ENABLE, w!("同步工作流（选择归档目录）…"))?;
    AppendMenuW(menu, MF_STRING, DISABLE, w!("停止保存后自动同步"))
}
impl Application {
    pub(in crate::desktop::application) fn queue_saved_workflow(&mut self) {
        if let Some(record) = self.restored_session.as_mut() {
            if let Some(settings) = record.workflow_settings.as_mut().filter(|s| s.enabled) {
                // 合并明确的保存意图，只保留最新已提交 Source，不排队图片/请求副本。
                settings.pending = Some(record.original.clone());
                (self.execution_wake.callback())();
            }
        }
    }
    pub(in crate::desktop::application) unsafe fn pending_workflow_ui(&mut self) {
        self.pending_workflow_ui_with(
            |hwnd| crate::desktop::session_asset_dialog::choose_workflow(hwnd),
            |hwnd, message, warning| {
                MessageBoxW(
                    Some(hwnd),
                    &HSTRING::from(message),
                    w!("Otter"),
                    MB_OK
                        | if warning {
                            MB_ICONWARNING
                        } else {
                            MB_ICONINFORMATION
                        },
                );
            },
        );
    }
    unsafe fn pending_workflow_ui_with(
        &mut self,
        mut choose: impl FnMut(HWND) -> Result<Option<Root>>,
        mut notify: impl FnMut(HWND, &str, bool),
    ) {
        if self.modals.blocked() || self.exit_requested.get() {
            return;
        }
        let mut request = None;
        for doc in &self.documents {
            if let Some(enabled) = doc.state.workflow_sync_requested.take() {
                request = Some((doc.window.0, doc.state.file_dialog.clone(), enabled));
            }
        }
        if let Some((hwnd, flag, enabled)) = request {
            if !enabled {
                if let Some(settings) = self
                    .restored_session
                    .as_mut()
                    .and_then(|r| r.workflow_settings.as_mut())
                {
                    settings.enabled = false;
                    settings.pending = None;
                }
                notify(
                    hwnd,
                    "已停止后续保存触发的同步。当前已启动的同步将继续完成。",
                    false,
                );
            } else {
                let result = (|| -> Result<()> {
                    if self.save_status() != save_state::Status::Unchanged
                        || self.catalog.borrow().loaded.is_none()
                    {
                        return Err(Error::new(E_ABORT, "请先保存工作区并连接 Loom 能力目录。"));
                    }
                    // 文件夹对话框会泵消息；选择结果只能授权打开前的工作区和目录身份。
                    let receipt = Receipt {
                        source: self.restored_session.as_ref().unwrap().original.clone(),
                        guard: guard::Guard::capture(self)?,
                        catalog: self.catalog.borrow().loaded.clone(),
                    };
                    let dialog = crate::desktop::file_dialog::Guard::begin(&flag)
                        .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
                    let selected = choose(hwnd)?;
                    drop(dialog);
                    let Some(root) = selected else {
                        return Ok(());
                    };
                    if !IsWindow(Some(hwnd)).as_bool() || !receipt.matches(self) {
                        return Err(Error::new(
                            E_ABORT,
                            "选择目录期间工作区或 Loom 连接已变化，请重新发起同步。",
                        ));
                    }
                    let root = Arc::new(root);
                    self.request_workflow_cycle(root.clone(), timestamp()?)?;
                    self.restored_session.as_mut().unwrap().workflow_settings = Some(Settings {
                        root,
                        enabled: true,
                        pending: None,
                        interactive: true,
                    });
                    Ok(())
                })();
                if let Err(error) = result {
                    notify(hwnd, &format!("无法启动工作流同步。\n{error}"), true);
                }
            }
        }
        if self
            .restored_session
            .as_ref()
            .is_none_or(|r| r.workflow_settings.is_none())
        {
            return;
        }
        if let Some(outcome) = self.take_workflow_cycle() {
            let interactive = self
                .restored_session
                .as_mut()
                .unwrap()
                .workflow_settings
                .as_mut()
                .unwrap()
                .interactive;
            self.restored_session
                .as_mut()
                .unwrap()
                .workflow_settings
                .as_mut()
                .unwrap()
                .interactive = false;
            let success =
                outcome.current && outcome.sync.as_ref().is_ok_and(|r| r.failure.is_none());
            let recovery = outcome
                .archive
                .as_ref()
                .and_then(|p| p.retained_recovery.as_ref());
            if interactive || !success || recovery.is_some() {
                let mut message = if success {
                    "工作流同步完成。".to_owned()
                } else {
                    format!("工作流同步未完整完成，不会自动重试。\n{:?}", outcome.sync)
                };
                if let Some((path, error)) = recovery {
                    message.push_str(&format!(
                        "\n归档已保存，恢复文件尚未清理：{}\n{error}",
                        path.display()
                    ));
                }
                if let Some(doc) = self.documents.first() {
                    notify(doc.window.0, &message, !success || recovery.is_some());
                }
            }
        }
        if self
            .restored_session
            .as_ref()
            .is_some_and(|r| r.workflow_cycle.is_some())
        {
            return;
        }
        let pending = self.restored_session.as_mut().and_then(|record| {
            let settings = record.workflow_settings.as_mut()?;
            settings
                .pending
                .take()
                .map(|source| (source, settings.root.clone(), settings.enabled))
        });
        if let Some((source, root, enabled)) = pending {
            if enabled
                && self
                    .restored_session
                    .as_ref()
                    .is_some_and(|r| r.original.same_storage(&source))
                && self.save_status() == save_state::Status::Unchanged
            {
                let result = timestamp().and_then(|time| self.request_workflow_cycle(root, time));
                if let Err(error) = result {
                    if let Some(doc) = self.documents.first() {
                        notify(doc.window.0, &format!("保存后同步未启动。\n{error}"), true);
                    }
                }
            }
        }
    }
}
fn timestamp() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .ok_or_else(|| Error::new(E_FAIL, "Cannot obtain workflow archive timestamp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_request_does_not_queue_workflow_intent() {
        let state = State::new(None, false).unwrap();
        assert!(unsafe { request(HWND::default(), &state, ENABLE) }.is_err());
        assert!(state.workflow_sync_requested.get().is_none());
        assert!(handles(ENABLE) && handles(DISABLE));
        assert!(!handles(0));
    }
}
