//! Shared validation for snapshots and deltas; a transaction copy clones at most 256 Arcs, not large observation values.
use super::*;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Default)]
pub struct Observations {
    values: BTreeMap<String, Arc<Observation>>,
}
#[derive(Deserialize)]
struct Snapshot {
    observations: Vec<Observation>,
}
impl Observations {
    /// Only for authenticated attach snapshots; missing observations are not an empty snapshot.
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, Error> {
        let snapshot: Snapshot = parse(bytes).map_err(|_| Error::Json)?;
        if snapshot.observations.len() > MAX_OBSERVATIONS {
            return Err(Error::Limit);
        }
        let mut state = Self::default();
        for value in snapshot.observations {
            value.validate()?;
            if state
                .values
                .insert(value.observation_id.clone(), Arc::new(value))
                .is_some()
            {
                return Err(Error::Duplicate);
            }
        }
        Ok(state)
    }
    pub fn get(&self, id: &str) -> Option<&Observation> {
        self.values.get(id).map(AsRef::as_ref)
    }
    pub fn iter(&self) -> impl Iterator<Item = &Observation> {
        self.values.values().map(AsRef::as_ref)
    }
    /// Accept only observation payloads; callers must route the complete batch and must not ignore other business events.
    pub fn apply<'a>(&mut self, payloads: impl IntoIterator<Item = &'a [u8]>) -> Result<(), Error> {
        let mut next = self.clone();
        for payload in payloads {
            let value = Observation::parse(payload)?;
            let expected = match next.values.get(&value.observation_id) {
                Some(old) => old.sequence.checked_add(1).ok_or(Error::Sequence)?,
                None => 1,
            };
            if value.sequence != expected {
                return Err(Error::Sequence);
            }
            if !next.values.contains_key(&value.observation_id)
                && next.values.len() >= MAX_OBSERVATIONS
            {
                return Err(Error::Limit);
            }
            next.values
                .insert(value.observation_id.clone(), Arc::new(value));
        }
        *self = next;
        Ok(())
    }
}
