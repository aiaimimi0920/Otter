//! Bounded UIA observation state machine; commit only after real ack, stabilize after two identical samples, and retain the original start on stable heartbeats.
use crate::{
    live_relay_observation::{Confidence, Observation, Source, State, MAX_OBSERVATIONS},
    live_source_uia::{root_error, Sample},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};
const MAX_SNAPSHOT_BYTES: usize = 512 * 1024;

// Measure the exact compact encoding without allocating a second snapshot or a JSON buffer.
struct Budget(usize);
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or(io::ErrorKind::InvalidData)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[derive(Default)]
pub struct Tracks(BTreeMap<String, Observation>);
impl Tracks {
    pub fn candidates(&self, samples: Vec<Sample>, now: u64) -> Result<Vec<Observation>, ()> {
        if samples.len() > MAX_OBSERVATIONS
            || now == 0
            || self.0.values().any(|o| o.observed_at_ms > now)
        {
            return Err(());
        }
        let failed = samples
            .iter()
            .any(|s| s.id == "uia:source" && s.value.is_err());
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        let mut total = self.0.len();
        for sample in samples {
            if !seen.insert(sample.id.clone()) {
                return Err(());
            }
            let old = self.0.get(&sample.id);
            if old.is_none() {
                total += 1;
                if total > MAX_OBSERVATIONS {
                    return Err(());
                }
            }
            if let Some(value) = Self::next(old, sample, now)? {
                values.push(value);
            }
        }
        let (state, reason) = if failed {
            (State::Error, "uia_scan_failed")
        } else {
            (State::Stale, "element_not_found")
        };
        for (id, previous) in &self.0 {
            if seen.contains(id)
                || (previous.state == state && previous.reason.as_deref() == Some(reason))
            {
                continue;
            }
            let mut value = previous.clone();
            value.sequence = value.sequence.checked_add(1).ok_or(())?;
            value.state = state;
            value.reason = Some(reason.into());
            value.value = None;
            value.stable_since_ms = None;
            value.observed_at_ms = now;
            values.push(value);
        }
        // Local accumulated snapshots are also bounded so per-round new/stale metadata cannot exceed the HTTP/Viewer 1MiB contract.
        let mut next: BTreeMap<&str, &Observation> = self
            .0
            .iter()
            .map(|(id, value)| (id.as_str(), value))
            .collect();
        for value in &values {
            value.validate().map_err(|_| ())?;
            next.insert(&value.observation_id, value);
        }
        if next.len() > MAX_OBSERVATIONS {
            return Err(());
        }
        serde_json::to_writer(Budget(MAX_SNAPSHOT_BYTES), &next).map_err(|_| ())?;
        Ok(values)
    }
    fn next(
        old: Option<&Observation>,
        sample: Sample,
        now: u64,
    ) -> Result<Option<Observation>, ()> {
        if old.is_some_and(|o| now < o.observed_at_ms) {
            return Err(());
        }
        let (state, value, reason, stable_since_ms) = match sample.value {
            Ok(value) => {
                let same = old.is_some_and(|o| {
                    o.value.as_ref() == Some(&value) && o.locator.as_ref() == Some(&sample.locator)
                });
                if old.is_some_and(|o| {
                    same && o.state == State::Stable && now - o.observed_at_ms < 1000
                }) {
                    return Ok(None);
                }
                (
                    if same {
                        State::Stable
                    } else {
                        State::Observing
                    },
                    Some(value),
                    None,
                    if same {
                        old.and_then(|o| o.stable_since_ms).or(Some(now))
                    } else {
                        None
                    },
                )
            }
            Err(reason) => {
                if old
                    .is_some_and(|o| o.state == State::Error && o.reason.as_ref() == Some(&reason))
                {
                    return Ok(None);
                }
                (State::Error, None, Some(reason), None)
            }
        };
        Ok(Some(Observation {
            observation_id: sample.id,
            sequence: old
                .map_or(Some(1), |o| o.sequence.checked_add(1))
                .ok_or(())?,
            state,
            source: Source::UiAutomation,
            confidence: Confidence::Exact,
            observed_at_ms: now,
            stable_since_ms,
            locator: Some(sample.locator),
            value,
            reason,
        }))
    }
    pub fn commit(&mut self, value: Observation) {
        self.0.insert(value.observation_id.clone(), value);
    }
    pub fn scan_error(&self, now: u64) -> Result<Vec<Observation>, ()> {
        self.candidates(vec![root_error("uia_scan_failed")], now)
    }
}
#[cfg(test)]
#[path = "live_source_observation_tests.rs"]
mod tests;
