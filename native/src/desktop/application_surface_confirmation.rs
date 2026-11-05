//! 显式菜单请求驱动原生确认与决定任务；消息泵返回后再次检查退出/服务切换。
use super::*;
use crate::desktop::composition::relay::sessions::ConfirmationAction;
use crate::surface_decision_runtime::{Error as DecisionError, Evidence};
#[path = "application_batch_confirmation.rs"]
mod batch_confirmation;
#[path = "application_batch_resume.rs"]
mod batch_resume;

impl Application {
    pub(super) fn confirm_surface_event(&mut self, id: &str) {
        if self.modals.blocked() || self.exit_requested.get() {
            return;
        }
        let origin = {
            let view = self.authorization.borrow();
            if view.closed || view.busy {
                return;
            }
            view.origin.clone()
        };
        let Some(origin) = origin else {
            return;
        };
        let flag = std::rc::Rc::new(std::cell::Cell::new(false));
        self.modals.register(&flag);
        let Some(_guard) = super::super::file_dialog::Guard::begin(&flag) else {
            return;
        };
        let mut failure = None;
        let result = self
            .remote_sessions
            .dispatch_surface_confirmation(id, |mut receipt| {
                let prepared = (|| {
                    if receipt.event.binding().origin() != origin {
                        return Err(Error::new(E_ABORT, "Surface service changed"));
                    }
                    let now = super::super::surface_windows::now_ms()?;
                    let lease = self
                        .authorization_owner
                        .lease(&origin, now)
                        .map_err(|error| {
                            Error::new(E_ABORT, format!("当前授权不可用：{error:?}"))
                        })?;
                    if lease.device_id() != receipt.event.binding().device() {
                        return Err(Error::new(E_ABORT, "Surface device changed"));
                    }
                    let config = crate::device_authorization_config::Config::from_env(&origin)
                        .map_err(|error| Error::new(E_FAIL, format!("配置无效：{error:?}")))?;
                    Ok((config, lease))
                })();
                let (config, lease) = match prepared {
                    Ok(value) => value,
                    Err(error) => {
                        failure = Some(error.to_string());
                        return ConfirmationAction::Keep(receipt);
                    }
                };
                let Some(window) = self
                    .surface_windows
                    .iter_mut()
                    .find(|window| window.binding() == receipt.event.binding())
                else {
                    failure = Some("Surface 窗口已不可用".into());
                    return ConfirmationAction::Keep(receipt);
                };
                let decision = match window.prompt_decision(&mut receipt) {
                    Ok(Some(decision)) => decision,
                    Ok(None) => return ConfirmationAction::Keep(receipt),
                    Err(error) => {
                        failure = Some(error.to_string());
                        return ConfirmationAction::Keep(receipt);
                    }
                };
                let now = super::super::surface_windows::now_ms();
                let view = self.authorization.borrow();
                let stopped = self.exit_requested.get()
                    || view.closed
                    || view.busy
                    || view.request.is_some()
                    || view.origin.as_deref() != Some(origin.as_str());
                drop(view);
                if stopped || now.is_err() {
                    receipt.decision = Some(Evidence {
                        decision,
                        revision: window.snapshot_revision(),
                        submission: Err(DecisionError::NotStarted),
                    });
                    failure = Some("确认后状态已变化，决定未提交".into());
                    return ConfirmationAction::Keep(receipt);
                }
                match window.begin_receipt_decision(receipt, decision, now.expect("checked clock"))
                {
                    Ok(input) => ConfirmationAction::Start(config, lease, input),
                    Err(returned) => {
                        failure = Some(returned.error.to_string());
                        let mut receipt = returned.receipt;
                        receipt.decision = Some(Evidence {
                            decision: returned.decision,
                            revision: window.snapshot_revision(),
                            submission: Err(DecisionError::NotStarted),
                        });
                        ConfirmationAction::Keep(receipt)
                    }
                }
            });
        self.authorization.borrow_mut().status = match result {
            Err(error) => format!("决定未启动：{error}"),
            Ok(true) => "正在提交明确选择的决定…".into(),
            Ok(false) => failure.unwrap_or_else(|| "未提交决定".into()),
        };
    }
}
