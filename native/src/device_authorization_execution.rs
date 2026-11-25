//! 执行停止与认证撤销分离；只为同一串行 owner 留下有界清理机会。
use super::*;

/// 停止方仅持有取消能力，不借用或转移原凭据。
pub(crate) struct ExecutionStop(Arc<State>);
impl ExecutionStop {
    pub(crate) fn stop(&self) {
        self.0.stop_execution();
    }
}

impl State {
    pub(super) fn stop_execution(&self) {
        self.execution_cancel.cancel();
        self.execution_watch.revoke();
    }
}

impl Lease {
    /// 宿主停止立即中断执行网络/child 等待，但不是撤销认证，也不允许恢复执行。
    pub(crate) fn stop_execution(&self) {
        self.state.stop_execution();
    }
    pub(crate) fn execution_stop_handle(&self) -> ExecutionStop {
        ExecutionStop(self.state.clone())
    }
    pub(crate) fn execution_stopped(&self) -> bool {
        self.is_revoked() || self.state.execution_cancel.is_stopped()
    }
    pub(crate) fn execution_notification(&self) -> std::io::Result<Notification> {
        self.state.execution_watch.notification()
    }
    /// 执行与清理仍串行，不新建凭据或租约；授权撤销同时取消两个网络信号。
    pub(crate) fn execution_control<'a>(
        &'a self,
        client: &'a Client,
    ) -> Result<Control<'a>, Error> {
        if self.execution_stopped() {
            return Err(Error::Revoked);
        }
        Ok(Control::new(
            client,
            self.online_session()?,
            &self.state.execution_cancel,
        ))
    }
}
