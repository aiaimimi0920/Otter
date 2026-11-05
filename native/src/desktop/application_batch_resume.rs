//! 显式菜单接续；执行前复核授权/窗口，不重新批准、不弹窗或自动重放前缀。
use super::*;
use crate::desktop::composition::relay::sessions::BatchResumeAction;
impl Application {
    pub(in crate::desktop::application) fn resume_surface_batch(&mut self, id: &str) {
        if self.modals.blocked() || self.exit_requested.get() {
            return;
        }
        let origin = {
            let view = self.authorization.borrow();
            if view.closed || view.busy || view.request.is_some() {
                return;
            }
            view.origin.clone()
        };
        let Some(origin) = origin else { return };
        let mut failure = None;
        let result = self.remote_sessions.dispatch_batch_resume(id, |receipt| {
            let prepared = (|| {
                let event = receipt
                    .events
                    .first()
                    .ok_or_else(|| Error::new(E_ABORT, "Empty Surface batch"))?;
                if event.binding().origin() != origin {
                    return Err(Error::new(E_ABORT, "Surface service changed"));
                }
                let now = crate::desktop::surface_windows::now_ms()?;
                let lease = self
                    .authorization_owner
                    .lease(&origin, now)
                    .map_err(|error| Error::new(E_ABORT, format!("当前授权不可用：{error:?}")))?;
                if lease.device_id() != event.binding().device() {
                    return Err(Error::new(E_ABORT, "Surface device changed"));
                }
                let config = crate::device_authorization_config::Config::from_env(&origin)
                    .map_err(|error| Error::new(E_FAIL, format!("配置无效：{error:?}")))?;
                Ok((config, lease, now))
            })();
            let (config, lease, now) = match prepared {
                Ok(value) => value,
                Err(error) => {
                    failure = Some(error.to_string());
                    return BatchResumeAction::Keep(Box::new(receipt));
                }
            };
            let Some(window) = self
                .surface_windows
                .iter_mut()
                .find(|window| window.binding() == receipt.events[0].binding())
            else {
                failure = Some("Surface 窗口已不可用".into());
                return BatchResumeAction::Keep(Box::new(receipt));
            };
            // refresh 可能同步关闭窗口，调用期间不能持有 View 的 RefCell 借用。
            if let Err(error) = window.batch_continuation_for(&receipt, now) {
                failure = Some(error.to_string());
                return BatchResumeAction::Keep(Box::new(receipt));
            }
            let changed = {
                let view = self.authorization.borrow();
                self.exit_requested.get()
                    || view.closed
                    || view.busy
                    || view.request.is_some()
                    || view.origin.as_deref() != Some(origin.as_str())
            };
            if changed {
                failure = Some("状态已变化，接续未启动".into());
                return BatchResumeAction::Keep(Box::new(receipt));
            }
            match window.begin_batch_continuation(receipt, now) {
                Ok(input) => BatchResumeAction::Start(config, lease, Box::new(input)),
                Err(returned) => {
                    failure = Some(returned.error.to_string());
                    BatchResumeAction::Keep(Box::new(returned.receipt))
                }
            }
        });
        self.authorization.borrow_mut().status = match result {
            Ok(true) => "正在继续批次的剩余操作…".into(),
            Ok(false) => failure.unwrap_or_else(|| "没有可继续的批次".into()),
            Err(error) => format!("接续未启动：{error}"),
        };
    }
}
