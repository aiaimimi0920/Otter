//! Unified task reaping after the application main loop ends; preserve order: stop notifications, stop capture, then reap background execution.
use super::*;
impl Application {
    pub(super) fn shutdown(&mut self) {
        crate::desktop::capture_countdown::cancel();
        unsafe {
            self.chrome.close();
        }
        self.history_window.take();
        crate::desktop::device_authorization_ui::share::shutdown(&self.authorization);
        self.capture_history.shutdown();
        self.local_loom.shutdown();
        self.locale_remote.shutdown();
        self.shortcut_remote.shutdown();
        // Do not wait for network or driver cleanup in individual-window WM_CLOSE callbacks.
        self.execution_wake.close();
        self.extension_commands.shutdown();
        unsafe {
            self.extension_shortcuts.shutdown();
        }
        self.extensions.shutdown();
        self.local_arts.shutdown();
        self.projection_edit.shutdown();
        self.account_projection.shutdown();
        self.reset_projection_inbox();
        self.projection_receive
            .shutdown(&mut self.projection_cleanup);
        self.live_freeze_job.take();
        {
            let mut view = self.authorization.borrow_mut();
            view.closed = true;
            view.consent.finish_save();
            view.request = None;
            view.busy = false;
            view.catalog_busy = false;
            view.mount_busy = false;
            view.catalog = None;
        }
        // Retain authorization for one bounded deactivation attempt, then revoke it and run existing forced cleanup regardless of success.
        if let Err(error) = self.finish_surface_transitions() {
            self.error.get_or_insert(error);
        }
        self.authorization_owner.close();
        if let Err(error) = self.shutdown_live() {
            self.error.get_or_insert(error);
        }
        self.remote_sessions.finish_projection(); // Even if drain fails, reap original Accept first, then preserve its revocation responsibility.
        self.collect_projection_abandoned(); // drain closes ordinary UI Event delivery but must not lose the original newly created Accept intent.
        if let Err(error) = self.flush_projection_sender() {
            self.error.get_or_insert(error);
        }
        self.projection_profile.finish_open();
        self.projection_profile.refresh();
        self.projection_cleanup.resolve(&self.projection_profile);
        if let Err(error) = self.projection_cleanup.flush() {
            self.error.get_or_insert_with(|| {
                Error::new(
                    E_FAIL,
                    format!("Projection cleanup remains unpersisted: {error:?}"),
                )
            });
        }
        // Return completed owners even if drain fails; clock anomalies use the invalidation instant without restoring display.
        let now = crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX);
        if let Err(error) = self
            .surface_windows
            .settle_shutdown(&mut self.remote_sessions, now)
        {
            self.error.get_or_insert(error);
        }
        if let Err(error) = self.persist_surface_recovery() {
            self.error.get_or_insert(error);
        }
        self.catalog.borrow_mut().job.shutdown();
        self.input_jobs.shutdown();
        self.retired_edits.shutdown();
        self.projection_profile.close();
        for report in self.execution_jobs.shutdown() {
            let unresolved = report.completion.cleanup_unconfirmed();
            if unresolved && self.error.is_none() {
                self.error = Some(Error::new(
                    E_FAIL,
                    "Native execution cleanup remains unconfirmed",
                ));
            }
        }
    }
}
