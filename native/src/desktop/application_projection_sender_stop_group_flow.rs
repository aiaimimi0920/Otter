//! 先逐项持久受理再沿原公平stop调度排空；取消只丢未受理项，持久请求可跨实例恢复。
use super::*;
impl Application {
    pub(in crate::desktop) unsafe fn start_projection_stop_group(
        &mut self,
        selection: StopSelection,
    ) {
        if let Some(batch) = Batch::capture(self, selection) {
            self.stop_projection_sender();
            self.projection_sender.stop_group = Some(batch);
            self.process_projection_stop_group();
        } else {
            self.sender_status("设备组未停止：来源、目录或关联已变化");
        }
    }
    pub(in crate::desktop::application::projection_sender) fn cancel_projection_stop_group(
        &mut self,
    ) {
        if let Some(mut batch) = self.projection_sender.stop_group.take() {
            batch.cancel();
            batch.report(self);
        }
    }
    pub(in crate::desktop::application::projection_sender) unsafe fn process_projection_stop_group(
        &mut self,
    ) {
        let Some(mut batch) = self.projection_sender.stop_group.take() else {
            return;
        };
        if !batch.matches(self) || self.exit_requested.get() {
            batch.cancel();
            batch.report(self);
            return;
        }
        for entry in &mut batch.entries {
            let before = entry.state;
            if let Some(request) = &entry.request {
                if request.published_retired()
                    && self
                        .workspace_checkpoint
                        .as_ref()
                        .and_then(|owner| owner.sender_index())
                        .is_some_and(|index| {
                            index.is_for(&batch.canonical)
                                && index.for_intent(request.intent()).is_none()
                        })
                {
                    entry.state = State::Stopped;
                } else if request.stop_requested() {
                    entry.state = State::Pending;
                } else if entry.state == State::Starting && !self.projection_sender.stop.pending() {
                    entry.state = State::Failed;
                }
            }
            batch.dirty |= before != entry.state;
        }
        if !self.projection_sender.stop.pending() && self.sender_stop_available() {
            while let Some(entry) = batch
                .entries
                .iter_mut()
                .find(|entry| entry.state == State::Queued)
            {
                // 仅对固定canonical中的原关联受理；Request/身份已在capture核验，未携带网络授权。
                if let Some(request) = entry
                    .request
                    .as_ref()
                    .filter(|_| self.projection_sender_inclusion(&entry.binding))
                {
                    self.begin_sender_stop_request(request.clone());
                    entry.state = if self.projection_sender.stop.pending() {
                        State::Starting
                    } else {
                        State::Failed
                    };
                } else {
                    entry.state = State::Failed;
                }
                batch.dirty = true;
                if self.projection_sender.stop.pending() {
                    break;
                }
            }
        }
        batch.report(self);
        if batch.entries.iter().any(|entry| {
            matches!(
                entry.state,
                State::Queued | State::Starting | State::Pending
            )
        }) {
            self.projection_sender.stop_group = Some(batch);
        } else {
            self.sender_status("设备组停止处理结束；逐项确认结果见菜单");
        }
    }
}
