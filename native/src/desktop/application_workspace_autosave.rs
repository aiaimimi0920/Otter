//! 显式工作区自动保存；Live 仅在模型变化时取时间点，共用单槽且不触发外部工作流。
use super::*;
use std::{cell::Cell, sync::atomic::AtomicBool};
#[path = "application_workspace_autosave_smoke.rs"]
pub(super) mod smoke;

#[derive(Default)]
pub(in crate::desktop) struct Control {
    enabled: Cell<bool>,
    generation: Cell<u64>,
}
impl Control {
    pub(in crate::desktop) fn enabled(&self) -> bool {
        self.enabled.get()
    }
    pub(in crate::desktop) fn toggle(&self) -> Result<()> {
        let Some(next) = self.generation.get().checked_add(1) else {
            self.disable(); // 即使原本开启，也不得在代际耗尽后继续沿用许可。
            return Err(Error::new(
                E_ABORT,
                "Workspace autosave generation exhausted",
            ));
        };
        self.generation.set(next);
        self.enabled.set(!self.enabled());
        Ok(())
    }
    pub(super) fn disable(&self) {
        self.enabled.set(false);
        self.generation.set(self.generation.get().saturating_add(1));
    }
}

impl Application {
    pub(super) unsafe fn pending_workspace_save_with(
        &mut self,
        mut notify: impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if self.modals.blocked() {
            return;
        }
        self.pending_checkpoint_with(&mut notify);
        if self.checkpoint_pending() || self.checkpoint_exit_waiting() {
            return;
        }
        let result = self.poll_workspace_autosave();
        self.workspace_autosave_result(result, &mut notify);
        if self.workspace_autosave_pending.is_some() {
            return; // 显式保存/另存为意图留在原槽；自动任务回收后由原 UI 路径消费。
        }
        let mut manual_warning = false;
        self.pending_save_ui_with(|app, origin, message, warning| {
            manual_warning |= warning;
            notify(app, origin, message, warning);
        });
        if manual_warning {
            // 手动保存失败/取消的目标不由后台自动重新尝试。
            self.workspace_autosave.disable();
            return;
        }
        let result = self.start_workspace_autosave();
        self.workspace_autosave_result(result, &mut notify);
        self.start_checkpoint_with(&mut notify);
    }
    unsafe fn workspace_autosave_result(
        &mut self,
        result: Result<()>,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Err(error) = result {
            self.workspace_autosave.disable();
            self.exit_requested.set(false);
            if let Some(doc) = self.documents.iter().find(|doc| doc.is_alive()) {
                notify(
                    self,
                    &doc.state.unit_id,
                    &format!("工作区自动保存已停止；当前内容仍保留。\n{error}"),
                    true,
                );
            }
        }
    }
    unsafe fn start_workspace_autosave(&mut self) -> Result<()> {
        if !self.workspace_autosave.enabled()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_after_save
            || self.open_after_save.is_some()
            || self.exit_requested.get()
            || self
                .documents
                .iter()
                .any(|doc| doc.state.open_session_requested.get())
        {
            return Ok(());
        }
        if self.save_requires_destination(false) {
            return Err(Error::new(E_ABORT, "请先保存工作区，再开启自动保存。"));
        }
        if !save_snapshot::automatic_ready(self)? {
            return Ok(());
        }
        if self
            .saved_state
            .as_ref()
            .is_some_and(|stamp| stamp.matches_automatic(self))
        {
            return Ok(());
        }
        if !restore_batch::guard::is_idle(self) {
            return Ok(()); // 等手势/后台任务的正常完成消息，不丢草稿、不新增定时器。
        }
        self.request_save_existing()?;
        self.workspace_autosave_pending = Some(self.workspace_autosave.generation.get());
        Ok(())
    }
    unsafe fn poll_workspace_autosave(&mut self) -> Result<()> {
        let Some(generation) = self.workspace_autosave_pending else {
            return Ok(());
        };
        let current = self.workspace_autosave.enabled()
            && self.workspace_autosave.generation.get() == generation
            && !self.exit_requested.get();
        if !current {
            self.cancel_save_candidate();
        }
        let result = self.poll_save_candidate();
        if matches!(result, Ok(save_control::Poll::Pending)) {
            return Ok(());
        }
        self.workspace_autosave_pending = None;
        PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
        let result = if current {
            result
        } else {
            Ok(save_control::Poll::Stale)
        };
        match result? {
            save_control::Poll::Ready(prepared) => {
                if !save_snapshot::automatic_ready(self)? {
                    return Ok(());
                }
                let published = prepared.commit_ready(self, &AtomicBool::new(false))?;
                if let Some((path, error)) = published.retained_recovery {
                    return Err(Error::new(
                        E_FAIL,
                        format!("工作区已保存；旧恢复副本保留：{}\n{error}", path.display()),
                    ));
                }
            }
            // 编辑使旧快照失效是正常路径；下次空闲只保存当前快照，不重放旧候选。
            save_control::Poll::Stale => {}
            save_control::Poll::Pending => unreachable!(),
        }
        Ok(())
    }
    pub(super) fn automatic_save_pending(&self) -> bool {
        self.workspace_autosave_pending.is_some() || self.checkpoint_pending()
    }
    pub(super) unsafe fn workspace_autosave_blocks_exit(&self) -> bool {
        self.workspace_autosave_pending.is_some()
    }
}

#[cfg(test)]
#[path = "application_workspace_autosave_tests.rs"]
mod tests;
