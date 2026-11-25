//! Frozen Source-observation submissions; reconcile unknown writes by exact read-only checks, never resend POST for lagging snapshots.
use super::*;
use crate::live_relay_observation::Observation;
use std::sync::atomic::Ordering;
pub(crate) struct ObservationSubmission<'a> {
    value: &'a Observation,
    previous: u64,
    attempted: bool,
}
impl<'a> ObservationSubmission<'a> {
    pub(crate) fn new(publication: &Publication, value: &'a Observation) -> Result<Self, Error> {
        if !publication.observing {
            return Err(Error::Input);
        }
        value.validate().map_err(|_| Error::Input)?;
        let previous = publication.actor.load(Ordering::Acquire);
        previous.checked_add(1).ok_or(Error::Cursor)?;
        Ok(Self {
            value,
            previous,
            attempted: false,
        })
    }
}
impl Control<'_> {
    fn source_snapshot(&self, publication: &Publication) -> Result<(u64, Value), Error> {
        let actual =
            self.read_source_snapshot(publication, Instant::now() + Duration::from_secs(3))?;
        let cursor = &actual["requesterControl"];
        let sequence = cursor["controlSequence"].as_u64().ok_or(Error::Cursor)?;
        let previous = publication.actor.load(Ordering::Acquire);
        if cursor["deviceId"] != self.session.device_id()
            || cursor["epoch"] != 1
            || sequence < previous
            || sequence > previous.checked_add(1).ok_or(Error::Cursor)?
        {
            return Err(Error::Cursor);
        }
        Ok((sequence, actual))
    }
    pub(super) fn source_cursor(&self, publication: &Publication) -> Result<(), Error> {
        let (sequence, _) = self.source_snapshot(publication)?;
        publication.actor.store(sequence, Ordering::Release);
        Ok(())
    }
    /// false only means the snapshot has not yet proved the write; callers retain the same Submission and use only GET afterward.
    pub(crate) fn publish_source_observation(
        &self,
        publication: &Publication,
        pending: &mut ObservationSubmission<'_>,
    ) -> Result<bool, Error> {
        if publication.actor.load(Ordering::Acquire) != pending.previous {
            return Err(Error::Cursor);
        }
        let sequence = pending.previous.checked_add(1).ok_or(Error::Cursor)?;
        if !pending.attempted {
            let payload = serde_json::to_value(pending.value).map_err(|_| Error::Input)?;
            let body = json!({"envelope":{"protocolVersion":protocol::PROTOCOL,"sessionId":publication.session,
            "epoch":1,"sequence":sequence,"messageType":"observation","payload":payload}});
            pending.attempted = true;
            let result = self.send(
                "POST",
                &path(publication.session(), Some("observations"))?,
                &serde_json::to_vec(&body).map_err(|_| Error::Input)?,
                Duration::from_secs(3),
            );
            match result {
                Ok(bytes) => {
                    let ack: Value = serde_json::from_slice(&bytes).map_err(|_| Error::Input)?;
                    if ack["protocolVersion"] != protocol::PROTOCOL
                        || ack["acceptedSequence"] != sequence
                        || ack["acceptedObservationSequence"] != pending.value.sequence
                    {
                        return Err(Error::Cursor);
                    }
                    publication.actor.store(sequence, Ordering::Release);
                    return Ok(true);
                }
                Err(error @ Error::Http(crate::device_session_client::Error::Connect)) => {
                    // Connect occurs only before HTTP/TLS sending; only this case permits another POST.
                    pending.attempted = false;
                    return Err(error);
                }
                Err(error) if error.retryable_read() => {}
                Err(error) => return Err(error),
            };
        }
        let (cursor, actual) = self.source_snapshot(publication)?;
        if cursor == sequence {
            // Same-lock snapshots must contain the complete identical payload; matching ID/sequence or request-created status alone never proves success.
            let values = actual["observations"].as_array().ok_or(Error::Input)?;
            let mut matches = values
                .iter()
                .filter(|v| v["observationId"] == pending.value.observation_id);
            let Some(value) = matches.next() else {
                return Err(Error::Cursor);
            };
            if matches.next().is_some()
                || Observation::parse(&serde_json::to_vec(value).map_err(|_| Error::Input)?)
                    .map_err(|_| Error::Input)?
                    != *pending.value
            {
                return Err(Error::Cursor);
            }
            publication.actor.store(sequence, Ordering::Release);
            return Ok(true);
        }
        Ok(false)
    }
}
