//! 应用主循环结束后的统一任务回收；保持停止通知、停止采集、最后回收后台执行的顺序。
use super::*;
impl Application {
    pub(super) fn shutdown(&mut self) {
        unsafe {
            self.chrome.close();
        }
        self.history_window.take();
        self.capture_history.shutdown();
        // 不在单个窗口 WM_CLOSE 回调中等待网络或驱动清理。
        self.execution_wake.close();
        self.local_arts.shutdown();
        self.projection_edit.shutdown();
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
        // 保留授权给一次有界停用；随后无论成功失败都撤权并执行原有强制回收。
        if let Err(error) = self.finish_surface_transitions() {
            self.error.get_or_insert(error);
        }
        self.authorization_owner.close();
        if let Err(error) = self.shutdown_live() {
            self.error.get_or_insert(error);
        }
        self.remote_sessions.finish_projection(); // drain 出错也先回收原 Accept，再保其减权责任。
        self.collect_projection_abandoned(); // drain 关闭普通 UI Event，但不能丢原新建 Accept 意图。
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
        // 即使 drain 失败也回交已完成的 owner；时钟异常采用失效时点，不恢复显示。
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
