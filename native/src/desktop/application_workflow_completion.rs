//! UI credential owner for sync results; network success and permission to write back to the current workspace are independent conditions.
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
    /// Recheck before each archive/source-plan use; even complete success of old messages cannot update a new workspace.
    pub(in crate::desktop::application) unsafe fn is_current(&self, app: &Application) -> bool {
        self.receipt.matches(app)
            && self
                .delivery
                .ready
                .matches_retained_source(&self.receipt.source)
    }
    /// Failed and stale results still expose partial-success reports; never pretend remotely confirmed state was rolled back.
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
    /// Transfer only the result owner, without source-writeback permission; retain and recheck Completion after extraction.
    pub(in crate::desktop::application) fn take_workflow_delivery(
        &mut self,
    ) -> Option<Result<Completion>> {
        self.restored_session.as_mut()?.workflow_delivery.take()
    }
}
