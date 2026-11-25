//! Recycle pruning triggered by loading/settings updates; use the existing save slot when idle, report failures once, and never retry in a loop.
use super::*;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_recycle_maintenance_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
impl Application {
    pub(in crate::desktop::application) fn schedule_recycle_maintenance(&mut self) {
        self.recycle_maintenance_pending = true;
        (self.execution_wake.callback())();
    }
    pub(in crate::desktop::application) unsafe fn start_recycle_maintenance(
        &mut self,
    ) -> Result<()> {
        if !self.recycle_maintenance_pending
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.modals.blocked()
            || self.exit_requested.get()
        {
            return Ok(());
        }
        // During editing/execution, wait for the next normal message to make progress; add no timers or background polling.
        let Ok(receipt) = self.capture_session_receipt() else {
            return Ok(());
        };
        self.recycle_maintenance_pending = false;
        let source = receipt.original().clone();
        let policy = self.recycle_policy;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .ok_or_else(|| Error::new(E_FAIL, "Invalid recycle maintenance clock"))?;
        let job = save_snapshot::job::Job::start_prune(
            source,
            policy,
            now_ms,
            self.execution_wake.callback(),
        )
        .map_err(|error| {
            Error::new(E_FAIL, format!("Cannot start recycle maintenance: {error}"))
        })?;
        self.save_job = Some((receipt, job));
        self.sync_frozen_menu();
        Ok(())
    }
}
