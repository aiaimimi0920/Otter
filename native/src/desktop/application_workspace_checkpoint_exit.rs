//! 正常退出的最终时间点；原单槽提交前不销毁 HWND，过期/失败不留下潜伏退出。
use super::*;

pub(super) struct Pending {
    intent: exit_intent::Intent,
    revision: u64,
}
impl Application {
    pub(in crate::desktop::application) fn checkpoint_exit_waiting(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.exit.is_some())
    }
    pub(super) unsafe fn checkpoint_exit_matches(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.exit.as_ref())
            .is_none_or(|pending| {
                restore_batch::guard::is_idle(self) && pending.intent.matches(self)
            })
    }
    pub(in crate::desktop::application) unsafe fn begin_checkpoint_exit(
        &mut self,
        intent: exit_intent::Intent,
    ) -> Result<bool> {
        let Some(owner) = self
            .workspace_checkpoint
            .as_ref()
            .filter(|owner| !owner.stopped)
        else {
            return Ok(false); // 无 profile/只读恢复副本没有新的写授权；沿明确退出选择。
        };
        let revision = owner
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new(E_ABORT, "Checkpoint revision exhausted"))?;
        if !intent.matches(self) || !self.start_checkpoint_mode(true)? {
            return Err(Error::new(
                E_ABORT,
                "工作区正忙或 Live 尚无可保存画面；当前窗口保留，请完成操作后重试退出。",
            ));
        }
        self.workspace_checkpoint.as_mut().unwrap().exit = Some(Pending { intent, revision });
        Ok(true)
    }
    /// Some(false) 也消费此次退出；不能让后来一次普通保存复活旧批准。
    pub(in crate::desktop::application) unsafe fn finish_checkpoint_exit(
        &mut self,
    ) -> Option<bool> {
        if !self.checkpoint_exit_waiting() {
            return None;
        }
        self.exit_requested.set(false); // 重复点击退出不使自己的 Receipt 过时。
        if self.checkpoint_pending() {
            return Some(false);
        }
        let owner = self.workspace_checkpoint.as_mut().unwrap();
        let pending = owner.exit.take().unwrap();
        let committed = !owner.stopped && owner.revision == pending.revision;
        Some(
            committed
                && self.save_job.is_none()
                && restore_batch::guard::is_idle(self)
                && pending.intent.matches(self),
        )
    }
}
