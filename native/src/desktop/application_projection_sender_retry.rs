//! 用户显式重试原 Request；不重新编码、签名或分配 projection ID，仍走原后台 Job。
use super::*;
use crate::{device_authorization_config::Config, projection_delivery_runtime::SenderInput};
impl Application {
    pub(in crate::desktop) unsafe fn retry_projection_sender(
        &mut self,
        unit: crate::workspace_id::UnitId,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let mut pending = match std::mem::take(&mut self.projection_sender.phase) {
            Phase::Retry(pending) => pending,
            phase => {
                self.projection_sender.phase = phase;
                return;
            }
        };
        let started = (|| {
            if pending.scope.unit != unit
                || !pending.scope.matches(self)
                || self.projection_sender.pending_local()
                || self.projection_sender.cleanup.network.is_some()
                || self.remote_sessions.projection_pending()
                || self.projection_receive.busy()
                || !self.sender_current(&pending.current, &pending.origin)
            {
                return Err(scope::stale());
            }
            let prepared = config(&pending.origin).map_err(|_| scope::stale())?;
            self.projection_sender
                .profile
                .input(prepared.identity_root())
                .ok_or_else(scope::stale)?;
            let current = self
                .authorization_owner
                .lease(&pending.origin, now()?)
                .map_err(|_| scope::stale())?;
            let lease = self
                .authorization_owner
                .lease(&pending.origin, now()?)
                .map_err(|_| scope::stale())?;
            lease.notify_revocation(self.execution_wake.callback());
            let request = pending.request.clone().ok_or_else(scope::stale)?;
            let source = pending.scope.binding_source.clone();
            let name = pending.name.clone();
            self.remote_sessions
                .send_projection(
                    prepared,
                    lease,
                    Box::new(move |_, track| {
                        track(request.clone());
                        Ok(SenderInput {
                            request,
                            source,
                            name,
                        })
                    }),
                )
                .map_err(|_| scope::stale())?;
            pending.current = current;
            Ok(())
        })();
        self.projection_sender.phase = if started.is_ok() {
            Phase::Waiting(pending)
        } else {
            Phase::Retry(pending)
        };
        self.sender_status(if started.is_ok() {
            "正在重试原投送邀请…"
        } else {
            "原邀请未重试；来源、授权或任务不可用"
        });
    }
}
