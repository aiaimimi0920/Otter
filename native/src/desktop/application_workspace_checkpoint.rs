//! 默认工作区持续检查点：共享原保存单槽/Receipt，只推进独立磁盘基线，不标记文件已保存。
use super::*;
use crate::{desktop::workspace_profile::Profile, session_source::Source};
use std::sync::atomic::AtomicBool;
#[path = "application_workspace_checkpoint_exit.rs"]
mod final_exit;
#[path = "application_projection_checkpoint_proof.rs"]
pub(super) mod projection_proof;
#[path = "application_workspace_scope.rs"]
mod scope;
#[path = "application_workspace_checkpoint_startup.rs"]
mod startup;
#[cfg(test)]
#[path = "application_workspace_checkpoint_tests.rs"]
mod tests;

pub(super) struct Owner {
    profile: Profile,
    original: Option<Source>,
    revision: u64,
    stamp: Option<(save_state::Stamp, Source)>,
    projections: Option<crate::projection_delivery::binding::checkpoint::Index>,
    senders: Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    projection_workspace: Option<crate::workspace::IdentityStamp>,
    projection_scope: Option<crate::session_projection_scope::Proof>,
    projection_display: projection_proof::Documents,
    pending: bool,
    stopped: bool,
    exiting: bool,
    exit: Option<final_exit::Pending>,
}
impl Owner {
    pub(in crate::desktop::application) fn profile_root(&self) -> &std::path::Path {
        self.profile.path.parent().unwrap()
    }
    pub(super) fn projection_deleted_binding(
        &self,
        workspace: &crate::workspace::Workspace,
        unit: &str,
    ) -> Option<&crate::projection_delivery::binding::Binding> {
        // 只授减权资格：普通编辑保工作区身份，整体交换不能借旧 checkpoint 删除新关联。
        if !self.projection_workspace.as_ref()?.matches(workspace) {
            return None;
        }
        self.projections.as_ref()?.for_unit(unit)
    }
    pub(super) fn projection_baseline(
        &self,
    ) -> Option<(
        &crate::projection_delivery::binding::checkpoint::Index,
        &Source,
    )> {
        Some((self.projections.as_ref()?, &self.stamp.as_ref()?.1))
    }
    pub(super) fn new(profile: Profile) -> Self {
        Self {
            profile,
            original: None,
            revision: 0,
            stamp: None,
            projections: None,
            senders: None,
            projection_workspace: None,
            projection_scope: None,
            projection_display: None,
            pending: false,
            stopped: false,
            exiting: false,
            exit: None,
        }
    }
}
impl Application {
    /// 仅供只读系统退出准入；必须是实际commit的完整当前检查点，不能借用在途Receipt。
    pub(super) unsafe fn session_checkpoint_current(&self) -> bool {
        self.workspace_checkpoint.as_ref().is_some_and(|owner| {
            !owner.stopped
                && !owner.pending
                && !owner.exiting
                && owner.exit.is_none()
                && owner.stamp.as_ref().is_some_and(|(stamp, source)| {
                    stamp.matches_automatic(self)
                        && self.restored_session.as_ref().is_some_and(|record| {
                            source.same_storage(&record.original)
                                && owner.projection_scope.as_ref().is_some_and(|proof| {
                                    // stamp的source是保留输入；Proof绑定真实落盘候选，不能混用两份身份。
                                    owner
                                        .original
                                        .as_ref()
                                        .is_some_and(|published| proof.is_for(published))
                                        && record.projection_scope == Some(proof.scope())
                                })
                        })
                })
        })
    }
    pub(super) fn checkpoint_is_exiting(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.exiting)
    }
    pub(super) fn projection_checkpoint_available(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| !owner.stopped && !owner.exiting && owner.exit.is_none())
    }
    pub(super) fn checkpoint_pending(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.pending)
    }
    pub(super) fn checkpoint_exiting(&mut self) {
        if let Some(owner) = &mut self.workspace_checkpoint {
            owner.exiting = true;
        }
    }
    pub(super) unsafe fn pending_checkpoint_with(
        &mut self,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Err(error) = self.poll_checkpoint() {
            self.stop_checkpoint(error, notify);
        }
    }
    pub(super) unsafe fn start_checkpoint_with(
        &mut self,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Err(error) = self.start_checkpoint() {
            self.stop_checkpoint(error, notify);
        }
    }
    unsafe fn stop_checkpoint(
        &mut self,
        error: Error,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Some(owner) = &mut self.workspace_checkpoint {
            owner.stopped = true;
            owner.exit = None;
        }
        let message = format!("工作区恢复检查点已停止；当前内容仍保留，请另存工作区。\n{error}");
        eprintln!("native-workspace-checkpoint-stopped: {error}");
        if let Some(doc) = self.documents.iter().find(|doc| doc.is_alive()) {
            notify(self, &doc.state.unit_id, &message, true);
        }
    }
    unsafe fn start_checkpoint(&mut self) -> Result<()> {
        self.start_checkpoint_mode(false).map(|_| ())
    }
    unsafe fn start_checkpoint_mode(&mut self, force: bool) -> Result<bool> {
        let Some(owner) = &self.workspace_checkpoint else {
            return Ok(false);
        };
        if owner.stopped
            || owner.exiting
            || owner.exit.is_some()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_after_save
            || self.open_after_save.is_some()
            || !restore_batch::guard::is_idle(self)
            || self
                .documents
                .iter()
                .any(|doc| doc.state.open_session_requested.get())
        {
            return Ok(false);
        }
        if !save_snapshot::automatic_ready(self)? {
            return Ok(false);
        }
        if !force
            && owner.projection_scope.is_some()
            && owner.stamp.as_ref().is_some_and(|(stamp, source)| {
                stamp.matches_automatic(self)
                    && self
                        .restored_session
                        .as_ref()
                        .is_some_and(|record| source.same_storage(&record.original))
            })
        {
            return Ok(false);
        }
        let destination = match &owner.original {
            Some(original) => {
                save_disk::Destination::Replace(owner.profile.path.clone(), original.clone())
            }
            None => save_disk::Destination::New(owner.profile.path.clone()),
        };
        // 已验证修订随磁盘基线一同推进；UI 热路径不重复解析整个检查点。
        let revision = owner.revision;
        let (receipt, data) = self.capture_session_snapshot()?;
        let job = save_snapshot::job::Job::start_checkpoint(
            data,
            destination,
            revision,
            self.execution_wake.callback(),
        )
        .map_err(|error| Error::new(E_FAIL, format!("Cannot start checkpoint: {error}")))?;
        self.save_job = Some((receipt, job));
        self.workspace_checkpoint.as_mut().unwrap().pending = true;
        Ok(true)
    }
    unsafe fn poll_checkpoint(&mut self) -> Result<()> {
        if !self.checkpoint_pending() {
            return Ok(());
        }
        let cancelled = self.exit_requested.get();
        if cancelled {
            self.cancel_save_candidate();
        }
        let result = self.poll_checkpoint_candidate();
        if matches!(result, Ok(save_control::Poll::Pending)) {
            return Ok(());
        }
        self.workspace_checkpoint.as_mut().unwrap().pending = false;
        // 在途期间保留的 Live freeze/recycle 等意图须自行续进，不能等下一条偶然输入。
        PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
        if cancelled {
            return Ok(()); // 退出正常使 Receipt 过时；不篡改 exit flag 来绕过校验。
        }
        if let save_control::Poll::Ready(mut prepared) = result? {
            if !self.checkpoint_exit_matches() {
                return Ok(()); // 新显式意图不能被最终快照或下一轮关窗越过。
            }
            let disk = prepared
                .staged
                .take()
                .ok_or_else(|| Error::new(E_FAIL, "Missing checkpoint file"))?;
            if !disk.is_for(&prepared.candidate)
                || !prepared.receipt.matches(self)
                || prepared
                    .checkpoint_senders
                    .as_ref()
                    .is_some_and(|index| !index.is_for(&prepared.candidate.bytes.clone().into()))
                || prepared.checkpoint_scope.as_ref().is_none_or(|proof| {
                    !proof.is_for(&prepared.candidate.bytes.clone().into())
                        || self.restored_session.as_ref().unwrap().projection_scope
                            != Some(proof.scope())
                })
                || prepared
                    .checkpoint_projections
                    .as_ref()
                    .is_some_and(|index| !index.is_for(&prepared.candidate.bytes.clone().into()))
            {
                return Err(Error::new(E_ABORT, "Checkpoint changed before commit"));
            }
            if !save_snapshot::automatic_ready(self)? {
                return Ok(()); // 新的 Live 手势/显式意图使后台时间点正常过期。
            }
            let stamp = save_state::Stamp::capture_snapshot(self, &prepared.receipt)?;
            let display = projection_proof::capture(self, prepared.checkpoint_projections.as_ref());
            let source = self.restored_session.as_ref().unwrap().original.clone();
            // 不泵 UI；成功前不改变 profile 基线，成功后不触碰手动文件/dirty/workflow。
            let published = disk.commit(&AtomicBool::new(false)).map_err(|failure| {
                Error::new(E_FAIL, format!("Cannot commit checkpoint: {failure:?}"))
            })?;
            self.projection_receive
                .committed(prepared.checkpoint_projections.as_ref(), &self.workspace);
            self.projection_edit.committed(
                prepared.checkpoint_scope.as_ref(),
                &prepared.candidate.bytes.clone().into(),
                &self.workspace,
            );
            self.projection_sender
                .committed(prepared.checkpoint_senders.as_ref(), &self.workspace);
            self.projection_sender.stop_committed(
                prepared.checkpoint_senders.as_ref(),
                &prepared.candidate.bytes.clone().into(),
                &source,
                &self.workspace,
            );
            let owner = self.workspace_checkpoint.as_mut().unwrap();
            owner.revision = prepared.candidate.revision;
            owner.original = Some(prepared.candidate.bytes.into());
            owner.stamp = Some((stamp, source));
            // 先记真实发布事实；旧恢复副本未退休的独立错误不能抹掉已经提交的包含关系。
            owner.projections = prepared.checkpoint_projections;
            owner.senders = prepared.checkpoint_senders;
            owner.projection_scope = prepared.checkpoint_scope;
            owner.projection_workspace = Some(self.workspace.identity_stamp());
            owner.projection_display = display;
            if let Some((path, error)) = published.retained_recovery {
                return Err(Error::new(
                    E_FAIL,
                    format!("检查点已写入；旧恢复副本保留：{}\n{error}", path.display()),
                ));
            }
        }
        Ok(())
    }
    /// 删除最后 Unit 后 WM_QUIT 已排队；在 Wake 关闭前完成空工作区，不把应用退出写为空。
    pub(super) unsafe fn finish_deleted_workspace(&mut self) -> Result<()> {
        if self
            .workspace_checkpoint
            .as_ref()
            .is_none_or(|owner| owner.stopped || owner.exiting)
        {
            return Ok(());
        }
        self.collect_closed();
        if !self.documents.is_empty() {
            return Ok(());
        }
        // 最后来源已销毁，手动/库/恢复候选均不能再发布；沿原 Job Drop 取消并回收。
        self.remote_sessions.finish_projection();
        self.collect_projection_abandoned();
        self.flush_projection_sender()?;
        self.projection_receive
            .shutdown(&mut self.projection_cleanup);
        self.projection_profile.finish_open();
        self.projection_profile.refresh(); // direct 终态 join 也须同步本次新建意图，不能以旧 entries 剪掉。
        self.projection_cleanup.resolve(&self.projection_profile);
        self.projection_cleanup.flush().map_err(|error| {
            Error::new(
                E_FAIL,
                format!("Cannot persist projection cleanup: {error:?}"),
            )
        })?;
        // 这里只是应用终态，不在单个 HWND 回调或仍有文档时同步等待编码。
        self.cancel_restore();
        self.restore_job.take();
        self.save_origin = None;
        self.open_after_save = None;
        self.exit_after_save = false;
        self.recycle_maintenance_pending = false;
        if !self.checkpoint_pending() {
            self.save_job.take();
        }
        loop {
            self.pending_windows(false)?; // 退休最后来源的输入/执行任务，完成仍由原 Wake 推进。
            self.poll_checkpoint()?;
            self.start_checkpoint()?;
            if !self.checkpoint_pending() && restore_batch::guard::is_idle(self) {
                let owner = self.workspace_checkpoint.as_ref().unwrap();
                return if owner.stamp.as_ref().is_some_and(|(stamp, source)| {
                    stamp.matches_automatic(self)
                        && self
                            .restored_session
                            .as_ref()
                            .is_some_and(|record| source.same_storage(&record.original))
                }) {
                    Ok(())
                } else {
                    Err(Error::new(
                        E_ABORT,
                        "Empty workspace checkpoint is blocked by pending work",
                    ))
                };
            }
            let mut message = MSG::default();
            if GetMessageW(&mut message, None, 0, 0).0 <= 0 {
                return Err(Error::new(
                    E_ABORT,
                    "Message loop ended before empty checkpoint",
                ));
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
