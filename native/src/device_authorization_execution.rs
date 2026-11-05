//! 本地 Art 的执行停止与认证撤销分离；只为同一串行 owner 留下有界 cancel 清理机会。
use super::*;

impl Lease {
    /// 宿主停止立即中断执行网络/child 等待，但不是撤销认证，也不允许恢复执行。
    pub(crate) fn stop_execution(&self) {
        self.state.execution_cancel.cancel();
        self.state.execution_watch.revoke();
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
