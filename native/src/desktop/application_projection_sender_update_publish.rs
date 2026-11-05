//! 更新回执只在原来源/Lease/canonical全部存活时交原Checkpoint；失败绝不补偿解绑稳定关联。
use super::*;
impl Application {
    pub(in crate::desktop::application::projection_sender) unsafe fn finish_sender_update(
        &mut self,
        result: std::result::Result<Output, crate::projection_delivery_runtime::Error>,
    ) {
        let Phase::Updating(pending) = std::mem::take(&mut self.projection_sender.phase) else {
            return;
        };
        let Pending {
            scope: captured,
            expected,
            request,
            current,
            stopped,
        } = *pending;
        let completed = (|| {
            let Ok(Output::SenderUpdated(mut ready)) = result else {
                return Err(scope::stale());
            };
            if stopped
                || !captured.matches(self)
                || !self.projection_sender_inclusion(&expected)
                || !ready.request().same_intent(&request)
                || !self.sender_current(&current, expected.envelope().origin())
                || !ready.authorized(&current, now()?)
            {
                return Err(scope::stale());
            }
            let candidate = ready.take_binding();
            if let Some(candidate) = candidate {
                if !candidate.expected().same_storage(&captured.binding_source)
                    || !candidate.index().is_for(candidate.source())
                    || !candidate.binding().matches_intent(request.intent())
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
                    current,
                    expected: candidate.binding().clone(),
                    source: candidate.source().clone(),
                    previous: captured.source.clone(),
                    workspace: self.workspace.identity_stamp(),
                    retained_disk: true,
                    updated: true,
                }));
            }
            Ok(())
        })();
        let success = completed.is_ok();
        self.projection_sender.updates.settle(
            &request,
            if success { Some(captured.into()) } else { None },
            Instant::now(),
        );
        self.sender_status(if success {
            "投送同步已核对；接收端呈现以回执为准"
        } else {
            "投送同步暂缓，保留当前内容与关联"
        });
    }
}
