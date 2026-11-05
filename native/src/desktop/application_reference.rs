//! 收藏复用保存快照/worker 单槽，完成后只提交库原文；不伪装为磁盘保存成功。
use super::*;
impl Application {
    pub(in crate::desktop::application) fn new_recycle_request(
        &self,
        unit: crate::workspace_id::UnitId,
    ) -> Result<save_snapshot::job::RecycleRequest> {
        let (entry_id, created_at) = entry_identity()?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .ok_or_else(|| Error::new(E_FAIL, "Invalid recycle clock"))?;
        Ok(save_snapshot::job::RecycleRequest {
            unit,
            entry_id,
            created_at,
            policy: self.recycle_policy,
            now_ms,
        })
    }
    pub(in crate::desktop::application) unsafe fn request_frozen_remove(
        &mut self,
        choice: crate::desktop::frozen_library_ui::Choice,
    ) -> Result<()> {
        if !choice.remove
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_requested.get()
            || self
                .restored_session
                .as_ref()
                .is_none_or(|record| !record.original.same_storage(&choice.source))
        {
            return Err(Error::new(
                E_ABORT,
                "Frozen removal source changed or workspace busy",
            ));
        }
        let (receipt, data) = self.capture_session_snapshot()?;
        drop(data); // 删除只需要编辑凭证和原文，不让 worker 持有或编码图片。
        let job = save_snapshot::job::Job::start_library_remove(
            choice.source,
            choice.kind,
            choice.entry_id,
            self.execution_wake.callback(),
        )
        .map_err(|error| Error::new(E_FAIL, format!("Cannot start frozen removal: {error}")))?;
        self.save_job = Some((receipt, job));
        self.sync_frozen_menu();
        Ok(())
    }
    unsafe fn start_reference(
        &mut self,
        request: crate::desktop::reference_ui::Request,
    ) -> Result<()> {
        if self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_requested.get()
            || self
                .restored_session
                .as_ref()
                .is_none_or(|record| !record.original.same_storage(&request.source))
            || !self.documents.iter().any(|doc| {
                doc.is_alive()
                    && doc.state.unit_id == request.unit
                    && !doc.state.is_art()
                    && (request.action != crate::desktop::reference_ui::Action::Recycle
                        || doc.state.live_request.borrow().is_none())
                    && request.target.matches(&doc.state.canvas.borrow())
            })
        {
            return Err(Error::new(E_ABORT, "Reference source or target changed"));
        }
        let (receipt, data) = self.capture_session_snapshot()?;
        let job = if request.action == crate::desktop::reference_ui::Action::Recycle {
            save_snapshot::job::Job::start_recycle(
                data,
                self.new_recycle_request(request.unit)?,
                self.execution_wake.callback(),
            )
        } else {
            let (id, timestamp) = entry_identity()?;
            save_snapshot::job::Job::start_reference(
                data,
                request.unit,
                request.action == crate::desktop::reference_ui::Action::Remove,
                id,
                timestamp,
                self.execution_wake.callback(),
            )
        }
        .map_err(|error| Error::new(E_FAIL, format!("Cannot start reference worker: {error}")))?;
        self.save_job = Some((receipt, job));
        Ok(())
    }
    unsafe fn poll_reference(&mut self) -> Result<()> {
        if !self
            .save_job
            .as_ref()
            .is_some_and(|(_, job)| job.is_library_update())
        {
            return Ok(());
        }
        let Some(result) = self.save_job.as_mut().and_then(|(_, job)| job.poll()) else {
            return Ok(());
        };
        let (receipt, job) = self.save_job.take().unwrap();
        if job
            .prune_policy()
            .is_some_and(|policy| self.recycle_policy != policy)
        {
            return Err(Error::new(E_ABORT, "Recycle policy changed during pruning"));
        }
        let output = result.map_err(|message| Error::new(E_ABORT, message))?;
        if output.staged.is_some()
            || output.candidate.expected_revision != output.candidate.revision
        {
            return Err(Error::new(E_ABORT, "Reference update cancelled or stale"));
        }
        // 成功的无变更维护不发布任何字节；首次呈现等模型变化不能把它变成阻塞宿主的失败。
        // worker取消、策略变化和异常候选仍在前面拒绝；真正写库仍复核完整Receipt。
        if !output.library_changed {
            return Ok(());
        }
        if !receipt.matches(self) || self.exit_requested.get() {
            return Err(Error::new(E_ABORT, "Reference update cancelled or stale"));
        }
        // Receipt 已验证来源/全工作区/坐标；最后核验后不泵消息，发布不触及 HWND 和磁盘。
        self.restored_session
            .as_mut()
            .unwrap()
            .edit_original(output.candidate.bytes.into());
        self.saved_state = None;
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn process_reference(
        &mut self,
        visible: bool,
    ) -> Result<()> {
        if self.modals.blocked() {
            return Ok(());
        }
        let result = self
            .poll_recycle(visible)
            .and_then(|()| self.poll_reference());
        self.sync_frozen_menu();
        self.reference_result(result, visible)?;
        let request = self.frozen_library.borrow_mut().reference.take();
        if let Some(request) = request {
            let result = self.start_reference(request);
            self.sync_frozen_menu();
            self.reference_result(result, visible)?;
        }
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn reference_result(
        &self,
        result: Result<()>,
        visible: bool,
    ) -> Result<()> {
        if let Err(error) = result {
            if !visible {
                return Err(error);
            }
            if let Some(owner) = self.documents.iter().find(|doc| doc.is_alive()) {
                MessageBoxW(
                    Some(owner.window.0),
                    &HSTRING::from(format!(
                        "无法完成冻结库操作，请检查贴纸和回收站状态。\n{error}"
                    )),
                    w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
        Ok(())
    }
}

fn entry_identity() -> Result<(String, String)> {
    let id = crate::desktop::document_state::new_unit_id()?;
    let now = unsafe { windows::Win32::System::SystemInformation::GetSystemTime() };
    Ok((
        id.as_str().into(),
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            now.wYear, now.wMonth, now.wDay, now.wHour, now.wMinute, now.wSecond, now.wMilliseconds
        ),
    ))
}

#[path = "application_reference_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
