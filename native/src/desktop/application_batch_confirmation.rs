//! 菜单显式选择驱动批次原生确认；模态返回后复核应用状态再消费提示。
use super::*;
use crate::desktop::composition::relay::sessions::BatchConfirmationAction;
impl Application {
    pub(in crate::desktop::application) fn confirm_surface_batch(&mut self, id: &str) -> bool {
        if self.modals.blocked() || self.exit_requested.get() {
            return false;
        }
        let origin = {
            let view = self.authorization.borrow();
            if view.closed || view.busy {
                return false;
            }
            view.origin.clone()
        };
        let Some(origin) = origin else { return false };
        let Ok(now) = crate::desktop::surface_windows::now_ms() else {
            return false;
        };
        if self
            .remote_sessions
            .pending_batch_confirmation_for(&origin, now, |receipt, index| {
                receipt.events[index].id() == id
            })
            .is_none()
        {
            return false;
        }
        let flag = std::rc::Rc::new(std::cell::Cell::new(false));
        self.modals.register(&flag);
        let Some(_guard) = crate::desktop::file_dialog::Guard::begin(&flag) else {
            return true;
        };
        let mut failure = None;
        let result = self
            .remote_sessions
            .dispatch_batch_confirmation(id, |receipt, index| {
                let configured = (|| {
                    let event = &receipt.events[index];
                    if event.binding().origin() != origin {
                        return Err(Error::new(E_ABORT, "Surface service changed"));
                    }
                    let now = crate::desktop::surface_windows::now_ms()?;
                    let lease = self
                        .authorization_owner
                        .lease(&origin, now)
                        .map_err(|error| {
                            Error::new(E_ABORT, format!("当前授权不可用：{error:?}"))
                        })?;
                    if lease.device_id() != event.binding().device() {
                        return Err(Error::new(E_ABORT, "Surface device changed"));
                    }
                    let config = crate::device_authorization_config::Config::from_env(&origin)
                        .map_err(|error| Error::new(E_FAIL, format!("配置无效：{error:?}")))?;
                    Ok((config, lease))
                })();
                let (config, lease) = match configured {
                    Ok(value) => value,
                    Err(error) => {
                        failure = Some(error.to_string());
                        return BatchConfirmationAction::Keep(Box::new(receipt));
                    }
                };
                let Some(window) = self
                    .surface_windows
                    .iter_mut()
                    .find(|window| window.binding() == receipt.events[index].binding())
                else {
                    failure = Some("Surface 窗口已不可用".into());
                    return BatchConfirmationAction::Keep(Box::new(receipt));
                };
                let choice = match window.prompt_batch_choice(&receipt, index) {
                    Ok(Some(choice)) => choice,
                    Ok(None) => return BatchConfirmationAction::Keep(Box::new(receipt)),
                    Err(error) => {
                        failure = Some(error.to_string());
                        return BatchConfirmationAction::Keep(Box::new(receipt));
                    }
                };
                let now = crate::desktop::surface_windows::now_ms();
                let stopped = {
                    let view = self.authorization.borrow();
                    self.exit_requested.get()
                        || view.closed
                        || view.busy
                        || view.request.is_some()
                        || view.origin.as_deref() != Some(origin.as_str())
                };
                if stopped || now.is_err() {
                    failure = Some("确认后状态已变化，决定未提交".into());
                    return BatchConfirmationAction::Keep(Box::new(receipt));
                }
                let now = now.expect("checked clock");
                let prepared = match window.prepare_batch_decision(receipt, index, choice, now) {
                    Ok(prepared) => prepared,
                    Err(returned) => {
                        failure = Some(returned.error.to_string());
                        return BatchConfirmationAction::Keep(Box::new(returned.receipt));
                    }
                };
                match window.begin_prepared_batch_decision(prepared, now) {
                    Ok(input) => BatchConfirmationAction::Start(config, lease, Box::new(input)),
                    Err(returned) => {
                        failure = Some(returned.error.to_string());
                        let prepared = returned.prepared;
                        let mut receipt = prepared.receipt;
                        // 已消费提示但未移交 owner：保存明确选择，不恢复成可重复决定的提示。
                        receipt
                            .observation
                            .as_mut()
                            .expect("prepared observation")
                            .as_mut()
                            .expect("prepared result")
                            .items[prepared.index]
                            .decision = Some(crate::surface_decision_runtime::Evidence {
                            decision: prepared.decision,
                            revision: receipt.revision,
                            submission: Err(crate::surface_decision_runtime::Error::NotStarted),
                        });
                        BatchConfirmationAction::Keep(Box::new(receipt))
                    }
                }
            });
        self.authorization.borrow_mut().status = match result {
            Err(error) => format!("批次决定未启动：{error}"),
            Ok(true) => "正在提交明确选择的批次决定…".into(),
            Ok(false) => failure.unwrap_or_else(|| "未提交决定".into()),
        };
        true
    }
}
