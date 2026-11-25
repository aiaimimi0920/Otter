//! Single-event delivery: restore the sole owner, then independently review presentation; callers must retain receipts.
use super::*;
use crate::{
    surface_action_ack::Ack,
    surface_click_event::ClickEvent,
    surface_event_runtime::{Error as EventError, Outcome as EventOutcome},
};

#[derive(Debug)]
pub enum PresentationError {
    /// Enqueue only to the original JS background owner; convert to Ok(Some(revision)) only after actual DOM-frame delivery.
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
    /// Retain convergence version or error; move Scene into the window once without caching another large scene.
    pub convergence: Option<std::result::Result<u64, EventError>>,
    pub delivery: std::result::Result<(), LeaseError>,
    /// Retain only execution output, not proof it was applied to the local graph; independent authorization is still required before use.
    pub commit: Option<crate::surface_result_commit::Commit>,
    pub confirmation: Option<std::result::Result<crate::surface_confirmation::Request, EventError>>,
    /// None means no successfully converged scene; only Ok(Some) means this snapshot was applied.
    pub presentation: std::result::Result<Option<u64>, PresentationError>,
}
impl EventReceipt {
    /// Only for bounded candidate selection, never a substitute for window, current-lease, or precommit authorization checks.
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
    /// Call only after explicit user choice. Failure retains the original receipt; success transfers only the prompt, not the owner.
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
    /// Transfer the sole owner after explicit user choice; reject stale scenes, revocation, and already attempted decisions.
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
    /// Recheck current-window authorization before reading prompts; successful reads in receipts are not long-lived UI permission.
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
    /// Identity mismatches return all results unchanged. After acceptance, ACK/errors accompany the receipt and are not discarded by the window.
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
        // Even without a presentable scene, promptly close HWNDs whose authorization was revoked.
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
                        // replace's final invalidation notification may fail; do not interpret new geometry with the old version.
                        let _ = self.close();
                        return Err(PresentationError::Window(error));
                    }
                    self.snapshot_revision = revision;
                    self.images.get_mut().clear();
                    *self.resources.get_mut() = Some(snapshot.resources);
                    // HWND calls may synchronously reenter; revocation after commit requires closing, not redisplaying.
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
