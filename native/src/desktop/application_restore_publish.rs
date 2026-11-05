//! 原生会话发布：所有可恢复的失败先于 owner 交换；不写文件、不丢未知会话字段。
use super::*;
#[path = "application_restore_publish_smoke.rs"]
pub(super) mod smoke;

pub(in crate::desktop::application) struct Record {
    pub restored_senders: Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    pub original: crate::session_source::Source,
    /// 原后台候选解析的只读关联；默认 profile 启动屏障领取，不等于普通导入已落盘。
    pub restored_projections: Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub restored_projection_scope: Option<crate::session_projection_scope::Proof>,
    /// 内存中只分配一次；没有 canonical checkpoint 的包含证明时，不授发送资格。
    pub projection_scope: Option<crate::session_projection_scope::Scope>,
    /// 库编辑可以改变内存原文；已有文件替换仍须比较最后一次真实提交的完整字节。
    pub disk_original: Option<crate::session_source::Source>,
    pub saved_path: Option<std::path::PathBuf>,
    pub recovery_path: Option<std::path::PathBuf>,
    pub space: Option<crate::session_placement::Space>,
    pub annotation_ids: Vec<(crate::workspace_id::UnitId, Vec<String>)>,
    /// 空会话的初始运行入口；后续写回须检查其当前内容，不能仅凭此 ID 永久排除用户编辑。
    pub transient_host: Option<crate::workspace_id::UnitId>,
    pub pending_arts: Vec<crate::workspace_id::UnitId>,
    pub binding_catalog: Option<Rc<Loaded>>,
    pub attempted_arts: Vec<crate::workspace_id::UnitId>,
    pub workflow: Option<super::workflow::Pending>,
    pub workflow_cycle: Option<super::workflow::cycle::Cycle>,
    pub workflow_settings: Option<super::workflow::ui::Settings>,
    pub workflow_sync: Option<super::workflow::sync::Sync>,
    pub workflow_delivery: Option<Result<super::workflow::sync::completion::Completion>>,
}

impl Record {
    pub(in crate::desktop::application) fn edit_original(
        &mut self,
        source: crate::session_source::Source,
    ) {
        if self.saved_path.is_some() && self.disk_original.is_none() {
            self.disk_original = Some(self.original.clone());
        }
        self.original = source;
        self.restored_projections = None;
        self.restored_senders = None;
        self.restored_projection_scope = None;
    }
}

impl Application {
    /// 调用方须在 UI 消息回调返回后执行，并已取得替换当前工作区的用户意图。
    /// visible=false 供隐藏 HWND 集成验证；成功后旧窗口 owner 才释放。
    pub unsafe fn publish_restored_batch(
        &mut self,
        mut batch: Windows,
        visible: bool,
    ) -> Result<()> {
        if !batch.matches_application(self) {
            return Err(Error::new(E_ABORT, "Restored workspace changed or busy"));
        }
        let transient_host = self.prepare_empty_restore_host(&mut batch)?;
        let groups = batch.candidate.workspace.groups().to_vec();
        let mut capture_windows = Vec::new();
        capture_windows
            .try_reserve_exact(batch.documents.len() * 2)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot prepare restored capture registry"))?;
        for doc in &batch.documents {
            capture_windows.extend([doc.window.0, doc.panel.0]);
            if visible {
                doc.observe_priority()?;
            }
        }
        // 安装监听后再次核验。此后直到交换只转交热键，不派发 UI 消息或执行节点。
        if !batch.matches_application(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed before restoration publication",
            ));
        }
        transfer_hotkey(&self.documents, &batch.documents, |doc, enabled| {
            crate::desktop::global_capture::set(doc.window.0, &doc.state, enabled)
        })?;
        let workspace = std::mem::take(&mut batch.candidate.workspace);
        let (saved_path, recovery_path) = match batch.candidate.origin.take() {
            Some(super::super::super::session_candidate::Origin {
                requested,
                recovery: None,
            }) => (Some(requested), None),
            Some(super::super::super::session_candidate::Origin {
                recovery: Some(path),
                ..
            }) => (None, Some(path)),
            None => (None, None),
        };
        let record = Record {
            restored_senders: batch.candidate.sender_index.take(),
            projection_scope: batch
                .candidate
                .projection_scope
                .as_ref()
                .map(|proof| proof.scope()),
            restored_projection_scope: batch.candidate.projection_scope.take(),
            restored_projections: batch.candidate.projection_index.take(),
            disk_original: None,
            workflow_cycle: None,
            workflow_settings: None,
            workflow_sync: None,
            workflow_delivery: None,
            workflow: None,
            pending_arts: batch
                .documents
                .iter()
                .filter(|doc| doc.state.restored_art && doc.state.selected_art.borrow().is_none())
                .map(|doc| doc.state.unit_id.clone())
                .collect(),
            binding_catalog: batch.catalog.clone(),
            attempted_arts: Vec::new(),
            original: batch.candidate.into_original_json(),
            saved_path,
            recovery_path,
            space: None,
            annotation_ids: batch.annotation_ids,
            transient_host,
        };
        // 以下为同步所有权交换：候选 HWND 中的 Box<State> 地址不变；不再调用可失败准备。
        self.stop_projection_sender();
        // 关闭旧 Art HWND 可重入；在完整旧工作区仍在位时撤权，不能插入所有权交换中途。
        self.local_arts.reset();
        let previous = std::mem::replace(&mut self.documents, batch.documents);
        self.workspace = workspace;
        self.workspace_autosave.disable();
        self.reset_projection_inbox();
        self.restored_session = Some(record);
        self.executions.clear();
        self.display_cache.borrow_mut().take();
        *self.display_composition.borrow_mut() = Default::default();
        *self.group_catalog.borrow_mut() = groups;
        *self.capture_windows.borrow_mut() = capture_windows;
        // 候选 State 的分组成员/锁定/隐藏状态已由 manifest 校验，不在交换期触发分组命令。
        if visible {
            for doc in &self.documents {
                if !doc.state.group_hidden.get() {
                    let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
                    if !doc.state.canvas.borrow().pinned {
                        let _ = ShowWindow(doc.panel.0, SW_SHOWNOACTIVATE);
                    }
                }
            }
        }
        drop(previous);
        // coordinator 尚未安装坐标上下文；不能沿用旧工作区的保存凭证。
        self.saved_state = None;
        if self
            .restored_session
            .as_ref()
            .is_some_and(|record| record.recovery_path.is_some())
        {
            println!("native-session-recovery: loaded=true save-as-required=true");
        }
        if visible {
            if let Some(path) = self
                .restored_session
                .as_ref()
                .and_then(|record| record.recovery_path.as_ref())
            {
                if let Some(document) = self.documents.first() {
                    windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                        Some(document.window.0),
                        &windows::core::HSTRING::from(format!("原会话文件缺失，已从恢复副本读取：\n{}\n请另存为确认恢复；恢复副本未被移动或删除。", path.display())),
                        windows::core::w!("Otter 会话恢复"),
                        windows::Win32::UI::WindowsAndMessaging::MB_OK | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
                    );
                }
            }
        }
        Ok(())
    }

    unsafe fn prepare_empty_restore_host(
        &self,
        batch: &mut Windows,
    ) -> Result<Option<crate::workspace_id::UnitId>> {
        if !batch.documents.is_empty() {
            return Ok(None);
        }
        batch
            .documents
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve empty workspace host"))?;
        batch
            .ready
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve empty workspace guard"))?;
        let document = self.prepare_window(State::new(None, false)?, false)?;
        let unit = document.state.unit_id.clone();
        batch
            .candidate
            .workspace
            .add_units([unit.clone()])
            .map_err(|error| {
                Error::new(
                    E_FAIL,
                    format!("Cannot register empty workspace host: {error:?}"),
                )
            })?;
        batch.ready.push(guard::Document::capture(&document)?);
        batch.documents.push(document);
        Ok(Some(unit))
    }
}

pub(in crate::desktop::application) fn transfer_hotkey(
    previous: &[DocumentWindow],
    next: &[DocumentWindow],
    mut change: impl FnMut(&DocumentWindow, bool) -> Result<()>,
) -> Result<()> {
    let Some(owner) = previous.iter().find(|doc| doc.state.global_capture.get()) else {
        return Ok(());
    };
    let target = next
        .first()
        .ok_or_else(|| Error::new(E_ABORT, "Missing restored hotkey owner"))?;
    let preferred = owner.state.hotkey_enabled.get();
    change(owner, false)?;
    if let Err(error) = change(target, true) {
        let rollback = change(owner, true);
        owner.state.hotkey_enabled.set(preferred);
        return match rollback {
            Ok(()) => Err(error),
            Err(rollback) => Err(Error::new(E_FAIL, format!(
                "Restoration hotkey transfer failed: {error}; original hotkey recovery failed: {rollback}"
            ))),
        };
    }
    target.state.hotkey_enabled.set(preferred);
    Ok(())
}
