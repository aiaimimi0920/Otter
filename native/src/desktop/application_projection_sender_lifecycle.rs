//! 授权/来源/工作区/退出的取消交接；稳定落盘绑定不因普通退出或换工作区被全量解绑。
use super::*;
impl Application {
    pub(in crate::desktop) fn stop_projection_sender(&mut self) {
        self.cancel_projection_group();
        self.cancel_projection_stop_group();
        self.projection_sender.updates.clear();
        let phase = std::mem::take(&mut self.projection_sender.phase);
        match phase {
            Phase::Waiting(mut pending) => {
                pending.stopped = true;
                self.remote_sessions.cancel_projection();
                self.projection_sender.phase = Phase::Waiting(pending);
            }
            Phase::Checkpoint(saved) => self.cancel_sender_saved(saved),
            Phase::Updating(mut pending) => {
                pending.stopped = true;
                self.remote_sessions.cancel_projection();
                self.projection_sender.phase = Phase::Updating(pending);
            }
            Phase::Retry(pending) => {
                if let Some(request) = pending.request {
                    self.projection_sender.cleanup.enqueue(request, false);
                }
            }
            Phase::Idle => {}
        }
    }
    pub(super) fn sender_saved_current(&self, saved: &Saved) -> bool {
        saved.workspace.matches(&self.workspace)
            && self.restore_job.is_none()
            && self.projection_checkpoint_available()
            && self
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == saved.expected.unit())
            && self.sender_current(&saved.current, saved.expected.envelope().origin())
    }
    pub(super) fn cancel_sender_saved(&mut self, saved: Box<Saved>) {
        if saved.retained_disk || self.projection_sender_inclusion(&saved.expected) {
            return;
        }
        let request = saved.ready.request();
        let same_workspace = saved.workspace.matches(&self.workspace);
        let rollback = self
            .restored_session
            .as_ref()
            .is_some_and(|record| record.original.same_storage(&saved.source));
        if same_workspace && !rollback {
            // 其他事务已改变完整原文，无法证明 binding 未保存；保原日志供显式恢复，不猜测解绑。
            return;
        }
        if same_workspace {
            self.restored_session
                .as_mut()
                .unwrap()
                .edit_original(saved.previous.clone());
            self.saved_state = None;
        }
        saved.ready.request_stop();
        drop(saved); // 先释放 Active/Slot，后续 CancelSend 才能重新 activate。
        self.projection_sender.cleanup.enqueue(request, false);
    }
    pub(in crate::desktop) fn flush_projection_sender(&mut self) -> Result<()> {
        self.stop_projection_sender();
        self.remote_sessions.finish_projection();
        self.collect_projection_sender();
        if let Phase::Waiting(pending) = std::mem::take(&mut self.projection_sender.phase) {
            if let Some(request) = pending.request {
                self.projection_sender.cleanup.enqueue(request, false);
            }
        }
        self.projection_sender.profile.collect(true);
        self.projection_sender
            .cleanup
            .resolve(&self.projection_sender.profile);
        self.projection_sender.cleanup.flush().map_err(|error| {
            Error::new(
                E_FAIL,
                format!("Sender cleanup remains unpersisted: {error:?}"),
            )
        })
    }
}
