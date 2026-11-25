//! Separate batch delivery and presentation: commit candidate scenes only after whole-batch success, moving all nonscene evidence into receipts.
use super::*;
use crate::{
    live_relay_http::{BatchStop, BatchSubmission},
    surface_click_event::ClickEvent,
    surface_event_batch_runtime::{Error as BatchError, Input, Observation, Outcome},
};
pub struct BatchReceipt {
    pub events: Vec<ClickEvent>,
    pub revision: u64,
    pub submission: std::result::Result<BatchSubmission, BatchError>,
    pub observation: Option<std::result::Result<Observation, BatchError>>,
    pub delivery: std::result::Result<(), LeaseError>,
    pub presentation: std::result::Result<Option<u64>, PresentationError>,
}
impl AuthorizedWindow {
    pub fn accept_batch(
        &mut self,
        mut outcome: Outcome,
        now_ms: u64,
    ) -> std::result::Result<BatchReceipt, Box<Outcome>> {
        let Input {
            owner,
            events,
            revision,
        } = outcome.input;
        if let Err(owner) = self.finish_batch(&events, owner) {
            outcome.input = Input {
                owner: *owner,
                events,
                revision,
            };
            return Err(Box::new(outcome));
        }
        // Scene does not enter the result cache; each event's version and commit remain in observation.items.
        let snapshot = outcome
            .observation
            .as_mut()
            .and_then(|result| result.as_mut().ok())
            .and_then(|observation| observation.snapshot.take());
        let refreshed = self.refresh(now_ms).map_err(PresentationError::Window);
        let mut presentation = match snapshot {
            None => refreshed.map(|()| None),
            Some(snapshot) => (|| {
                refreshed?;
                outcome.delivery.map_err(PresentationError::Authorization)?;
                self.lease
                    .validate(self.binding().origin(), now_ms)
                    .map_err(PresentationError::Authorization)?;
                if self.stopped.is_some()
                    || !self.is_shown()
                    || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
                {
                    return Err(PresentationError::Inactive);
                }
                let submitted = outcome
                    .submission
                    .as_ref()
                    .map_err(|_| PresentationError::Evidence)?;
                let observed = outcome
                    .observation
                    .as_ref()
                    .and_then(|value| value.as_ref().ok())
                    .ok_or(PresentationError::Evidence)?;
                if events.is_empty()
                    || submitted.stop != BatchStop::Complete
                    || submitted.acknowledgements.len() != events.len()
                    || observed.items.len() != events.len()
                    || !events
                        .iter()
                        .zip(&submitted.acknowledgements)
                        .zip(&observed.items)
                        .all(|((event, ack), item)| {
                            event.attempted()
                                && ack.matches_event(event)
                                && ack.succeeded()
                                && item.confirmation.is_none()
                                && item.result.is_ok()
                                && item.decision.as_ref().is_none_or(|evidence| {
                                    evidence.submission.is_ok()
                                        && evidence.decision.confirmed()
                                        && evidence.revision == self.snapshot_revision
                                        && evidence.decision.matches(event, ack)
                                })
                        })
                {
                    return Err(PresentationError::Evidence);
                }
                if revision != self.snapshot_revision
                    || events.iter().any(|event| event.revision() != revision)
                    || snapshot.revision < revision
                    || observed
                        .items
                        .iter()
                        .filter_map(|item| item.result.ok())
                        .max()
                        != Some(snapshot.revision)
                    || observed
                        .items
                        .iter()
                        .any(|item| item.result.is_ok_and(|value| value < revision))
                {
                    return Err(PresentationError::Revision);
                }
                snapshot
                    .resources
                    .validate(self.binding(), snapshot.revision)
                    .map_err(|_| PresentationError::Evidence)?;
                if self.javascript.is_some() {
                    return self.stage_javascript_snapshot(
                        snapshot.resources,
                        snapshot.revision,
                        now_ms,
                    );
                }
                if let Err(error) = self
                    .window
                    .replace(snapshot.scene.with_image_availability(|_| false))
                {
                    let _ = self.close();
                    return Err(PresentationError::Window(error));
                }
                self.snapshot_revision = snapshot.revision;
                self.images.get_mut().clear();
                *self.resources.get_mut() = Some(snapshot.resources);
                self.refresh(now_ms).map_err(PresentationError::Window)?;
                self.lease
                    .validate(self.binding().origin(), now_ms)
                    .map_err(PresentationError::Authorization)?;
                if !self.is_shown() {
                    return Err(PresentationError::Inactive);
                }
                Ok(Some(snapshot.revision))
            })(),
        };
        if matches!(presentation, Ok(Some(_))) {
            if let Err(error) = self.window.settle_edits(&events) {
                let _ = self.close();
                presentation = Err(PresentationError::Window(error));
            }
        } else if matches!(outcome.submission, Err(BatchError::NotStarted))
            && events.iter().all(|event| !event.attempted())
        {
            self.window.cancel_edits(&events);
        } else if !(matches!(presentation, Err(PresentationError::Pending(_)))
            || matches!(presentation, Ok(None))
                && outcome.delivery.is_ok()
                && outcome.submission.as_ref().is_ok_and(|submission| {
                    submission.stop == BatchStop::Confirmation
                        && submission.acknowledgements.len() <= events.len()
                        && submission
                            .acknowledgements
                            .iter()
                            .zip(&events)
                            .all(|(ack, event)| event.attempted() && ack.matches_event(event))
                        && submission.acknowledgements.last().is_some_and(|ack| {
                            ack.accepted()
                                && ack.status()
                                    == crate::surface_action_ack::Status::AwaitingConfirmation
                        })
                }))
        {
            // On prerequisite-edit failure/insufficient evidence, discard subsequent intents while retaining visible drafts and complete result evidence.
            // Explicit awaiting-confirmation is a suspended wait, not failure; confirmation/admission gates continue blocking sends.
            self.window.fail_edits(&events);
        }
        Ok(BatchReceipt {
            events,
            revision,
            submission: outcome.submission,
            observation: outcome.observation,
            delivery: outcome.delivery,
            presentation,
        })
    }
}
