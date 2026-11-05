//! 加载/设置更新触发的回收裁剪；空闲后使用现有保存单槽，失败报告一次，不循环重试。
use super::*;
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
        // 编辑/执行期间等下一次正常消息推进；不新增定时器或后台轮询。
        let Ok((receipt, data)) = self.capture_session_snapshot() else {
            return Ok(());
        };
        self.recycle_maintenance_pending = false;
        let source = data.original.clone();
        drop(data); // 裁剪不编码任何活动图片。
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
