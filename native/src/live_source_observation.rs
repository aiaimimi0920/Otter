//! 有界UIA观察状态机；仅真实ack后commit，两次相同样本稳定，稳定心跳保留原始起点。
use crate::{
    live_relay_observation::{Confidence, Observation, Source, State},
    live_source_uia::{root_error, Sample},
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
pub(crate) struct Tracks(BTreeMap<String, Observation>);
impl Tracks {
    pub(crate) fn candidates(
        &self,
        samples: Vec<Sample>,
        now: u64,
    ) -> Result<Vec<Observation>, ()> {
        if samples.len() > 256 || now == 0 || self.0.values().any(|o| o.observed_at_ms > now) {
            return Err(());
        }
        let failed = samples
            .iter()
            .any(|s| s.id == "uia:source" && s.value.is_err());
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        for sample in samples {
            if !seen.insert(sample.id.clone()) {
                return Err(());
            }
            if !self.0.contains_key(&sample.id)
                && self.0.len() + seen.iter().filter(|id| !self.0.contains_key(*id)).count() > 256
            {
                return Err(());
            }
            if let Some(value) = self.next(sample, now)? {
                values.push(value);
            }
        }
        for (id, previous) in &self.0 {
            if seen.contains(id) {
                continue;
            }
            let mut value = previous.clone();
            let state = if failed { State::Error } else { State::Stale };
            let reason = if failed {
                "uia_scan_failed"
            } else {
                "element_not_found"
            };
            if value.state == state && value.reason.as_deref() == Some(reason) {
                continue;
            }
            value.sequence = value.sequence.checked_add(1).ok_or(())?;
            value.state = state;
            value.reason = Some(reason.into());
            value.value = None;
            value.stable_since_ms = None;
            value.observed_at_ms = now;
            values.push(value);
        }
        // 本地累计快照也受限，避免逐轮新增/stale元数据突破HTTP/Viewer的1MiB合同。
        let mut next = self.0.clone();
        for value in &values {
            value.validate().map_err(|_| ())?;
            next.insert(value.observation_id.clone(), value.clone());
        }
        if next.len() > 256 || serde_json::to_vec(&next).map_err(|_| ())?.len() > 512 * 1024 {
            return Err(());
        }
        Ok(values)
    }
    fn next(&self, sample: Sample, now: u64) -> Result<Option<Observation>, ()> {
        let old = self.0.get(&sample.id);
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
    pub(crate) fn commit(&mut self, value: Observation) {
        self.0.insert(value.observation_id.clone(), value);
    }
    pub(crate) fn scan_error(&self, now: u64) -> Result<Vec<Observation>, ()> {
        self.candidates(vec![root_error("uia_scan_failed")], now)
    }
}
#[cfg(test)]
#[path = "live_source_observation_tests.rs"]
mod tests;
