//! 先核来源和授权，再消费后台候选；等待原 canonical commit，不把 HTTP 成功显示成接收端已呈现。
use super::*;
impl Application {
    pub(in crate::desktop) unsafe fn finish_projection_sender(
        &mut self,
        result: std::result::Result<Output, crate::projection_delivery_runtime::Error>,
        cleanup: bool,
    ) {
        if cleanup {
            if let Some(request) = self.projection_sender.cleanup.network.take() {
                let success =
                    matches!(result, Ok(Output::SenderUnlinked)) || request.published_retired();
                self.projection_sender.cleanup.finished(&request, success);
                if request.published_retired() {
                    self.projection_sender.profile.retired(&request);
                }
            }
            return;
        }
        if matches!(self.projection_sender.phase, Phase::Updating(_))
            || matches!(result, Ok(Output::SenderUpdated(_)))
        {
            self.finish_sender_update(result);
            return;
        }
        let phase = std::mem::take(&mut self.projection_sender.phase);
        let Phase::Waiting(pending) = phase else {
            if let Ok(Output::Sent(ready)) = result {
                self.projection_sender
                    .cleanup
                    .enqueue(ready.request(), false);
            }
            return;
        };
        let request = pending.request.clone();
        if result.is_err()
            && !pending.stopped
            && request.is_some()
            && pending.scope.matches(self)
            && self.sender_current(&pending.current, &pending.origin)
        {
            // 普通网络失败不等于用户取消；显式重试继续同 ID/签名/PNG，不自动重新 Create。
            self.projection_sender.phase = Phase::Retry(pending);
            self.sender_status("发送未完成；可重试原邀请或取消");
            return;
        }
        let prepared = (|| {
            let Ok(Output::Sent(mut ready)) = result else {
                return Err(scope::stale());
            };
            if pending.stopped
                || !pending.scope.matches(self)
                || !self.sender_current(
                    &pending.current,
                    ready.request().intent().envelope().origin(),
                )
                || !ready.authorized(&pending.current, now()?)
            {
                return Err(scope::stale());
            }
            let candidate = ready.take_binding().ok_or_else(scope::stale)?;
            if !candidate
                .expected()
                .same_storage(&pending.scope.binding_source)
                || !candidate.index().is_for(candidate.source())
                || !candidate.binding().matches_intent(ready.request().intent())
            {
                return Err(scope::stale());
            }
            self.restored_session
                .as_mut()
                .ok_or_else(scope::stale)?
                .edit_original(candidate.source().clone());
            self.saved_state = None;
            self.projection_sender.phase = Phase::Checkpoint(Box::new(Saved {
                ready: *ready,
                current: pending.current,
                expected: candidate.binding().clone(),
                source: candidate.source().clone(),
                previous: pending.scope.source,
                workspace: pending.scope.workspace,
                retained_disk: false,
                updated: false,
            }));
            Ok(())
        })();
        if prepared.is_err() {
            if let Some(request) = request {
                self.projection_sender.cleanup.enqueue(request, false);
            }
            self.sender_status("发送未完成；正在保留并清理原投送意图");
        } else {
            self.sender_status("邀请已创建，正在保存发送关联…");
        }
    }
    pub(in crate::desktop) unsafe fn poll_projection_sender(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        if self.projection_sender.profile.collect(false) {
            if let Some(entries) = self.projection_sender.profile.entries() {
                for request in entries
                    .iter()
                    .filter(|entry| entry.intent().cancelled() && !entry.stop_requested())
                {
                    self.projection_sender
                        .cleanup
                        .enqueue(request.clone(), true);
                }
            }
        }
        let phase = std::mem::take(&mut self.projection_sender.phase);
        match phase {
            Phase::Waiting(mut pending) => {
                if !pending.scope.matches(self)
                    || !self.sender_current(&pending.current, &pending.origin)
                {
                    pending.stopped = true;
                    self.remote_sessions.cancel_projection();
                }
                if !self.remote_sessions.projection_pending() {
                    if let Some(request) = pending.request {
                        self.projection_sender.cleanup.enqueue(request, false);
                    }
                    self.sender_status("发送已停止；未结意图保留");
                } else {
                    self.projection_sender.phase = Phase::Waiting(pending);
                }
            }
            Phase::Checkpoint(saved) => {
                if self.projection_sender_inclusion(&saved.expected) {
                    if let Some(batch) = &mut self.projection_sender.group {
                        batch.committed(&saved);
                    }
                    self.sender_status(if saved.updated {
                        "投送关联与当前版本已保存"
                    } else if saved.expected.linked() {
                        "发送关联已保存；接收端已接收"
                    } else {
                        "邀请已发送，等待接收回执"
                    });
                } else if !self.sender_saved_current(&saved) {
                    self.cancel_sender_saved(saved);
                    self.sender_status("发送已停止；未结意图保留");
                } else {
                    self.projection_sender.phase = Phase::Checkpoint(saved);
                }
            }
            Phase::Retry(pending) => {
                if pending.scope.matches(self)
                    && self.sender_current(&pending.current, &pending.origin)
                {
                    self.projection_sender.phase = Phase::Retry(pending);
                } else {
                    if let Some(request) = pending.request {
                        self.projection_sender.cleanup.enqueue(request, false);
                    }
                    self.sender_status("发送已停止；未结意图保留");
                }
            }
            Phase::Idle => {}
            Phase::Updating(pending) => self.poll_sender_update(pending),
        }
        self.poll_sender_stop();
        self.process_sender_cleanup(config);
        self.process_projection_group(config);
        self.process_sender_updates(config, std::time::Instant::now());
        let mut view = self.authorization.borrow_mut();
        view.sender.group_stopping = self.projection_sender.stop_group.is_some();
        view.sender.busy = self.projection_sender.busy();
        view.sender.cancelable = self.projection_sender.group.is_some()
            || self.projection_sender.stop_group.is_some()
            || matches!(
                self.projection_sender.phase,
                Phase::Waiting(_) | Phase::Retry(_)
            )
            || matches!(&self.projection_sender.phase, Phase::Checkpoint(saved) if !saved.updated);
        view.sender.stop_source = self
            .projection_sender_scope()
            .and_then(|_| self.workspace_checkpoint.as_ref()?.sender_source().cloned());
        view.sender.can_stop = self.sender_stop_available()
            && self.projection_sender.stop_group.is_none()
            && !self.projection_sender.stop.busy
            && self.projection_sender.profile.entries().is_some();
        view.sender.retry = match &self.projection_sender.phase {
            Phase::Retry(pending) => Some(pending.scope.unit.clone()),
            _ => None,
        };
        view.sender.ready = self.projection_sender.profile.entries().is_some()
            && self.projection_sender_scope().is_some()
            && !self.projection_sender.pending_local();
        view.sender.ready &= self.projection_sender.cleanup.network.is_none();
        view.sender.bindings = self
            .workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.sender_index())
            .map(|index| index.entries().to_vec())
            .unwrap_or_default();
    }
}
