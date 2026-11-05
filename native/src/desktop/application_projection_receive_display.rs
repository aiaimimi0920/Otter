//! 唯一接收状态的保存→呈现→认证回执；沿原 Job 移交 Hold，不复制槽、许可或网络 owner。
use super::*;
use crate::{
    device_authorization_config::Config,
    projection_delivery_runtime::{Job, Output},
};

impl Application {
    pub(super) fn receive_source_current(&self, source: &Source, unit: &str) -> bool {
        self.restore_job.is_none()
            && self
                .restored_session
                .as_ref()
                .is_some_and(|record| source.same_storage(&record.original))
            && self
                .workspace
                .units()
                .iter()
                .any(|current| current.as_str() == unit)
    }
    pub(super) unsafe fn display_projection_saved(
        &mut self,
        saved: Box<Saved>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<&'static str> {
        if !self.receive_source_current(&saved.source, saved.held.ticket().unit())
            || !self.receive_hold_valid(&saved.held, &saved.current)
        {
            return Err(Error::new(
                E_ABORT,
                "Projection saved destination changed before presentation",
            ));
        }
        if !saved.durable && !self.projection_checkpoint_available() {
            return Err(stale());
        }
        if !saved.durable {
            self.projection_receive.0 = Phase::Checkpoint(saved);
            return Ok("正在保存接收投送…");
        }
        // 模态期间不建新可见窗口；已有循环/消息会驱动后续结算，不增加计时器。
        if self.modals.blocked() {
            self.projection_receive.0 = Phase::Checkpoint(saved);
            return Ok("投送已保存，等待呈现");
        }
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == saved.held.ticket().unit())
            .ok_or_else(stale)?;
        saved.visual.present(doc)?;
        if !self.receive_source_current(&saved.source, saved.held.ticket().unit())
            || !self.receive_hold_valid(&saved.held, &saved.current)
            || self.projection_checkpoint_inclusion(saved.held.ticket()) != Some(&saved.expected)
            || (saved.recovery
                && !self.projection_recovery_current(saved.held.ticket(), &saved.expected))
            || !saved.visual.visible(doc)
        {
            return Err(Error::new(
                E_ABORT,
                "Projection saved proof changed after presentation",
            ));
        }
        let config = config(saved.held.origin()).map_err(|error| {
            Error::new(
                E_ABORT,
                format!("Projection displayed configuration failed: {error:?}"),
            )
        })?;
        let recovery = saved.recovery.then_some(saved.expected);
        let job = Job::start_displayed(config, saved.held).map_err(|error| {
            Error::new(
                E_FAIL,
                format!("Cannot start projection displayed receipt: {error}"),
            )
        })?;
        self.projection_receive.0 = Phase::Reporting(Box::new(Reporting {
            source: saved.source,
            current: saved.current,
            visual: saved.visual,
            job,
            recovery,
        }));
        Ok("投送已呈现，正在确认回执…")
    }
    pub(super) unsafe fn poll_projection_report(
        &mut self,
        mut reporting: Box<Reporting>,
    ) -> Result<&'static str> {
        let held = reporting.job.held().ok_or_else(stale)?;
        let valid = self.receive_source_current(&reporting.source, held.ticket().unit())
            && self.receive_hold_valid(held, &reporting.current)
            && reporting
                .recovery
                .as_ref()
                .is_none_or(|expected| self.projection_recovery_current(held.ticket(), expected))
            && self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id.as_str() == held.ticket().unit())
                .is_some_and(|doc| reporting.visual.visible(doc));
        if !valid {
            reporting.job.request_stop();
        }
        match reporting.job.try_join() {
            None => {
                self.projection_receive.0 = Phase::Reporting(reporting);
                Ok(if valid {
                    "投送已呈现，正在确认回执…"
                } else {
                    "正在取消投送回执…"
                })
            }
            Some(Ok(Output::Displayed)) if valid => {
                let envelope = reporting.job.held().unwrap().ticket().envelope();
                self.authorization
                    .borrow_mut()
                    .projection
                    .entries
                    .retain(|entry| entry.envelope() != envelope);
                Ok("投送已呈现")
            }
            // 只输出封闭错误类别；不自动重试/退休/解绑或格式化持有的内容。
            Some(Err(error)) => Err(Error::new(
                E_ABORT,
                format!(
                    "Projection displayed worker failed: {error:?}; destination_current={valid}"
                ),
            )),
            _ => Err(Error::new(
                E_ABORT,
                "Projection displayed completion lost its presentation proof",
            )),
        }
    }
}
