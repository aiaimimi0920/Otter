//! 单次事件交付：先恢复唯一 owner，再独立审核呈现；收据必须由调用方保存。
use super::*;
use crate::{
    surface_action_ack::Ack,
    surface_click_event::ClickEvent,
    surface_event_runtime::{Error as EventError, Outcome as EventOutcome},
};

#[derive(Debug)]
pub enum PresentationError {
    /// 仅排入原 JS 后台 owner；实际 DOM 帧交付后才可转成 Ok(Some(revision))。
    Pending(u64),
    Authorization(LeaseError),
    Inactive,
    Evidence,
    Revision,
    Window(Error),
}
pub struct EventReceipt {
    pub decision: Option<crate::surface_decision_runtime::Evidence>,
    pub event: ClickEvent,
    pub result: std::result::Result<Ack, EventError>,
    /// 保留收敛版本或错误；Scene 仅移入窗口一次，不缓存第二份大场景。
    pub convergence: Option<std::result::Result<u64, EventError>>,
    pub delivery: std::result::Result<(), LeaseError>,
    /// 仅保存执行输出，不代表输出已应用于本地图；使用前仍需独立授权。
    pub commit: Option<crate::surface_result_commit::Commit>,
    pub confirmation: Option<std::result::Result<crate::surface_confirmation::Request, EventError>>,
    /// None 表示没有成功收敛场景；Ok(Some) 才表示本次已经应用快照。
    pub presentation: std::result::Result<Option<u64>, PresentationError>,
}
impl EventReceipt {
    /// 仅用于有界候选选择；不能替代窗口、当前租约和提交前的授权检查。
    pub fn confirmation_ready(&self, now_ms: u64) -> bool {
        self.decision.is_none()
            && self.delivery.is_ok()
            && self.commit.is_none()
            && matches!(self.presentation, Ok(None))
            && !self
                .convergence
                .as_ref()
                .is_some_and(|result| result.is_ok())
            && self.result.as_ref().is_ok_and(|ack| {
                self.confirmation.as_ref().is_some_and(|result| {
                    result
                        .as_ref()
                        .is_ok_and(|request| request.validate(&self.event, ack, now_ms).is_ok())
                })
            })
    }
}
impl AuthorizedWindow {
    /// 仅在用户明确选择后调用。失败保留原收据；成功仅转移提示，不移交 owner。
    pub fn prepare_decision(
        &mut self,
        receipt: &mut EventReceipt,
        choice: crate::surface_confirmation::Choice,
        now_ms: u64,
    ) -> Result<crate::surface_confirmation::Decision> {
        self.confirmation_for(receipt, now_ms)?;
        if receipt.decision.is_some() {
            return Err(Error::new(E_FAIL, "Surface decision already exists"));
        }
        let ack = receipt.result.as_ref().expect("validated acknowledgement");
        let request = receipt
            .confirmation
            .take()
            .expect("validated confirmation")
            .expect("validated confirmation result");
        match crate::surface_confirmation::Decision::new(
            request,
            &receipt.event,
            ack,
            choice,
            now_ms,
        ) {
            Ok(decision) => Ok(decision),
            Err(failure) => {
                receipt.confirmation = Some(Ok(failure.request));
                Err(Error::new(E_FAIL, "Surface decision preparation failed"))
            }
        }
    }
    /// 用户已作出明确选择后移交唯一 owner；拒绝旧场景、撤权和已尝试决定。
    pub fn begin_decision(
        &mut self,
        event: &ClickEvent,
        ack: &Ack,
        decision: &crate::surface_confirmation::Decision,
        now_ms: u64,
    ) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.stopped.is_some() || !self.is_shown() || self.snapshot_revision != event.revision()
        {
            return Ok(None);
        }
        decision
            .request()
            .validate(event, ack, now_ms)
            .map_err(|_| Error::new(E_FAIL, "Surface decision became stale"))?;
        Ok(self.lifecycle.take_decision(event, ack, decision))
    }
    /// 读取提示前复核当前窗口授权；收据中的成功读取不是长期有效的 UI 许可。
    pub fn confirmation_for<'a>(
        &mut self,
        receipt: &'a EventReceipt,
        now_ms: u64,
    ) -> Result<&'a crate::surface_confirmation::Request> {
        self.refresh(now_ms)?;
        if self.stopped.is_some()
            || !self.is_shown()
            || receipt.delivery.is_err()
            || self.binding() != receipt.event.binding()
            || self.snapshot_revision != receipt.event.revision()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Err(Error::new(E_FAIL, "Surface confirmation is not authorized"));
        }
        let ack = receipt
            .result
            .as_ref()
            .map_err(|_| Error::new(E_FAIL, "Missing Surface acknowledgement"))?;
        let request = receipt
            .confirmation
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .ok_or_else(|| Error::new(E_FAIL, "Missing Surface confirmation"))?;
        request
            .validate(&receipt.event, ack, now_ms)
            .map_err(|_| Error::new(E_FAIL, "Surface confirmation became stale"))?;
        Ok(request)
    }
    /// 身份不匹配时原样退回全部结果。接纳后 ACK/错误随收据返回，不由窗口丢弃。
    pub fn accept_event(
        &mut self,
        outcome: EventOutcome,
        now_ms: u64,
    ) -> std::result::Result<EventReceipt, Box<EventOutcome>> {
        let EventOutcome {
            decision,
            owner,
            event,
            result,
            convergence,
            confirmation,
            delivery,
        } = outcome;
        if let Err(owner) = self.finish_event(&event, owner) {
            return Err(Box::new(EventOutcome {
                decision,
                owner: *owner,
                event,
                result,
                convergence,
                confirmation,
                delivery,
            }));
        }
        let summary = convergence.as_ref().map(|value| {
            value
                .as_ref()
                .map(|snapshot| snapshot.revision)
                .map_err(|error| *error)
        });
        // 即使没有可呈现场景，也应及时关闭已经撤权的 HWND。
        let refreshed = self.refresh(now_ms).map_err(PresentationError::Window);
        let mut commit = None;
        let presentation = match convergence {
            Some(Ok(mut snapshot)) => {
                commit = snapshot.commit.take();
                (|| {
                    refreshed?;
                    delivery.map_err(PresentationError::Authorization)?;
                    self.lease
                        .validate(self.binding().origin(), now_ms)
                        .map_err(PresentationError::Authorization)?;
                    if self.stopped.is_some()
                        || !self.is_shown()
                        || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
                    {
                        return Err(PresentationError::Inactive);
                    }
                    if !event.attempted()
                        || !result.as_ref().is_ok_and(|ack| {
                            ack.matches_event(&event)
                                && ack.succeeded()
                                && decision.as_ref().is_none_or(|evidence| {
                                    evidence.submission.is_ok()
                                        && evidence.decision.confirmed()
                                        && evidence.decision.matches(&event, ack)
                                        && evidence.revision == self.snapshot_revision
                                })
                        })
                    {
                        return Err(PresentationError::Evidence);
                    }
                    if event.revision() != self.snapshot_revision
                        || snapshot.revision < self.snapshot_revision
                    {
                        return Err(PresentationError::Revision);
                    }
                    let revision = snapshot.revision;
                    snapshot
                        .resources
                        .validate(self.binding(), revision)
                        .map_err(|_| PresentationError::Evidence)?;
                    if self.javascript.is_some() {
                        return self.stage_javascript_snapshot(
                            snapshot.resources,
                            revision,
                            now_ms,
                        );
                    }
                    if let Err(error) = self
                        .window
                        .replace(snapshot.scene.with_image_availability(|_| false))
                    {
                        // replace 的最后失效通知可能失败，不能继续用旧版本解释新几何。
                        let _ = self.close();
                        return Err(PresentationError::Window(error));
                    }
                    self.snapshot_revision = revision;
                    self.images.get_mut().clear();
                    *self.resources.get_mut() = Some(snapshot.resources);
                    // HWND 调用可能同步重入；提交后撤权必须关闭，不重新显示。
                    self.refresh(now_ms).map_err(PresentationError::Window)?;
                    self.lease
                        .validate(self.binding().origin(), now_ms)
                        .map_err(PresentationError::Authorization)?;
                    if !self.is_shown() {
                        return Err(PresentationError::Inactive);
                    }
                    Ok(Some(revision))
                })()
            }
            _ => refreshed.map(|()| None),
        };
        Ok(EventReceipt {
            decision,
            event,
            result,
            convergence: summary,
            delivery,
            commit,
            confirmation,
            presentation,
        })
    }
}
