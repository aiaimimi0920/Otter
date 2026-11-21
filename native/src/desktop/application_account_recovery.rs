//! 每次工作区/来源检查全部写前记录；失败意图保留，只有显式导入才重试未知接受。
use super::*;
impl Application {
    pub(super) unsafe fn start_account_recovery(&mut self) -> Result<bool> {
        let Some(scope) = self
            .restored_session
            .as_ref()
            .and_then(|r| r.projection_scope)
        else {
            return Ok(false);
        };
        let Some(origin) = self.authorization.borrow().origin.clone() else {
            return Ok(false);
        };
        let key = (scope, origin.clone());
        if self.account_projection.recovery.as_ref() == Some(&key) {
            if self
                .authorization
                .borrow()
                .consent
                .participation()
                .is_none()
            {
                return Ok(false);
            }
            if let Some(intent) = self.account_projection.recovery_queue.pop_front() {
                self.start_account_request(
                    crate::desktop::device_authorization_ui::account::Request::Recover(intent),
                )?;
                return Ok(true);
            }
            return Ok(false);
        }
        let participation = self
            .authorization
            .borrow()
            .consent
            .participation()
            .ok_or_else(stale)?;
        let Some(owner) = self.workspace_checkpoint.as_ref() else {
            return Ok(false);
        };
        let root = owner.profile_root().to_owned();
        let bindings = owner.account_bindings(&self.workspace);
        let destination = account_projection::Destination::capture_existing(self)?;
        let source = destination.source().clone();
        let units = self.workspace.units().to_vec();
        let lease = self
            .authorization_owner
            .lease(&origin, crate::desktop::surface_windows::now_ms()?)
            .map_err(|_| stale())?;
        let selected = origin.clone();
        let job = job::Job::start(lease, self.execution_wake.callback(), move |lease| {
            let cancel = lease.preparation_cancel();
            let mut store = account::journal::Store::open(&root, cancel).map_err(job::failure)?;
            let entries = store.entries().to_vec();
            let mut recover = Vec::new();
            for intent in entries {
                if intent.scope != scope || intent.loom_origin != selected {
                    continue;
                }
                if bindings
                    .iter()
                    .any(|binding| binding.stopped() && intent.matches(binding))
                {
                    // 仅已提交的停止记录允许退休；网络失败和取消不会走这里。
                    store.retire(&intent, cancel).map_err(job::failure)?;
                } else if !units.iter().any(|unit| unit.as_str() == intent.unit) {
                    recover.push(intent);
                }
            }
            Ok(Output::Recovery(recover))
        })?;
        self.account_projection.pending = Some(Pending {
            job,
            origin,
            source,
            workspace: self.workspace.identity_stamp(),
            guard: Guard::Receive(Box::new(destination)),
            cancel: None,
            participation: Some(participation),
        });
        self.account_status("正在核对账号关联…");
        Ok(true)
    }
}
