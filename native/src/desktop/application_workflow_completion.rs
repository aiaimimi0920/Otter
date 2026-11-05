//! 同步结果的 UI 凭证 owner；网络成功与允许写回当前工作区是两个独立条件。
use super::*;
pub(in crate::desktop::application) struct Completion {
    receipt: Receipt,
    delivery: WorkflowDelivery,
}
impl Completion {
    pub(in crate::desktop::application) fn into_report(
        self,
    ) -> crate::session_document::workflow_snapshot::batch::dispatch::Report {
        self.delivery.report
    }
    pub(in crate::desktop::application) fn new(
        receipt: Receipt,
        delivery: WorkflowDelivery,
    ) -> Self {
        Self { receipt, delivery }
    }
    /// 每次使用归档/来源计划前复核；即使旧消息全部成功，也不能更新新的工作区。
    pub(in crate::desktop::application) unsafe fn is_current(&self, app: &Application) -> bool {
        self.receipt.matches(app) && self.delivery.ready.matches_source(&self.receipt.source)
    }
    /// 失败和过期结果仍可读取部分成功报告，不把远端已确认状态伪装成回滚。
    pub(in crate::desktop::application) fn report(
        &self,
    ) -> &crate::session_document::workflow_snapshot::batch::dispatch::Report {
        &self.delivery.report
    }
    pub(in crate::desktop::application) unsafe fn current_ready(
        &self,
        app: &Application,
    ) -> Option<&WorkflowReady> {
        (self.is_current(app) && self.delivery.report.failure.is_none())
            .then_some(&self.delivery.ready)
    }
}
impl Application {
    /// 只转移结果 owner，不赋予来源写回权限；取出后必须仍保留并复核 Completion。
    pub(in crate::desktop::application) fn take_workflow_delivery(
        &mut self,
    ) -> Option<Result<Completion>> {
        self.restored_session.as_mut()?.workflow_delivery.take()
    }
}
