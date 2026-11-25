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
        let receipt = self.capture_session_receipt()?;
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
        use crate::desktop::reference_ui::Action;
        let clear = match request.action {
            Action::ClearRecycle => Some(crate::session_library::Kind::RecycleBin),
            Action::ClearReference => Some(crate::session_library::Kind::ReferenceLibrary),
            _ => None,
        };
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
                    && (clear.is_some() || !doc.state.is_art())
                    && (request.action != crate::desktop::reference_ui::Action::Recycle
                        || doc.state.live_request.borrow().is_none())
                    && request.target.matches(&doc.state.canvas.borrow())
            })
        {
            return Err(Error::new(E_ABORT, "Reference source or target changed"));
        }
        let wake = self.execution_wake.callback();
        let (receipt, job) = if let Some(kind) = clear {
            (
                self.capture_session_receipt()?,
                save_snapshot::job::Job::start_library_clear(request.source, kind, wake),
            )
        } else if request.action == Action::Recycle {
            let (receipt, data) = self.capture_frozen_snapshot(&request.unit)?;
            (
                receipt,
                save_snapshot::job::Job::start_recycle(
                    data,
                    self.new_recycle_request(request.unit)?,
                    wake,
                ),
            )
        } else {
            let (receipt, addition) = if request.action == Action::Set {
                let (receipt, data) = self.capture_frozen_snapshot(&request.unit)?;
                let (entry_id, created_at) = entry_identity()?;
                (
                    receipt,
                    Some(save_snapshot::job::ReferenceAddition {
                        data,
                        entry_id,
                        created_at,
                    }),
                )
            } else {
                (self.capture_session_receipt()?, None)
            };
            (
                receipt,
                save_snapshot::job::Job::start_reference(
                    request.source,
                    request.unit,
                    addition,
                    wake,
                ),
            )
        };
        let job = job.map_err(|error| {
            Error::new(E_FAIL, format!("Cannot start reference worker: {error}"))
        })?;
        self.save_job = Some((receipt, job));
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn poll_reference(&mut self) -> Result<()> {
        if !self
            .save_job
            .as_ref()
            .is_some_and(|(_, job)| job.is_library_update())
        {
            return Ok(());
        }
        let Some(result) = self
            .save_job
            .as_mut()
            .and_then(|(_, job)| job.poll_library_update())
        else {
            return Ok(());
        };
        let (receipt, job) = self.save_job.take().unwrap();
        if job
            .prune_policy()
            .is_some_and(|policy| self.recycle_policy != policy)
        {
            return Err(Error::new(E_ABORT, "Recycle policy changed during pruning"));
        }
        // 无变更不发布字节；策略变化和 worker 取消仍拒绝，真正写库仍复核完整 Receipt。
        let Some(candidate) = result.map_err(|message| Error::new(E_ABORT, message))? else {
            return Ok(());
        };
        if candidate.expected_revision != candidate.revision
            || !receipt.matches(self)
            || self.exit_requested.get()
        {
            return Err(Error::new(E_ABORT, "Reference update cancelled or stale"));
        }
        // Receipt 已验证来源/全工作区/坐标；最后核验后不泵消息，发布不触及 HWND 和磁盘。
        self.restored_session
            .as_mut()
            .unwrap()
            .edit_original(candidate.bytes.into());
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_library_clear_tests.rs"]
mod clear_tests;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_reference_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
