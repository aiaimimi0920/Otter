//! Remote-lease owner for each formal delivery; retain execution identity and never silently drop pending releases on failure.
use crate::{
    graph_requests::Request,
    loom_art_control::{self, Outcome},
    workspace_id::UnitId,
};
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidOutputs,
    InvalidHandle,
    Capacity,
    Allocation,
    Busy,
    Empty,
    Encoding,
}
pub struct Leases {
    unit: UnitId,
    execution: Request,
    handles: Vec<String>,
    attempt: Option<UnitId>,
}
impl Leases {
    /// Scan top-level formal-output resources; same-named fields in arbitrary value JSON are not releasable handles.
    pub fn outputs(unit: UnitId, execution: Request, outputs: &Value) -> Result<Self, Error> {
        let map = outputs.as_object().ok_or(Error::InvalidOutputs)?;
        if map.len() > 128 {
            return Err(Error::Capacity);
        }
        Self::collect(unit, execution, map.values())
    }
    pub fn preview(unit: UnitId, execution: Request, value: &Value) -> Result<Self, Error> {
        Self::collect(unit, execution, std::iter::once(value))
    }
    fn collect<'a>(
        unit: UnitId,
        execution: Request,
        values: impl Iterator<Item = &'a Value>,
    ) -> Result<Self, Error> {
        let mut handles = Vec::new();
        for value in values {
            if value["kind"].as_str() != Some("shared_memory") {
                continue;
            }
            let handle = value["handle"]
                .as_str()
                .filter(|name| {
                    name.starts_with("Loom_Buffer_")
                        && name.len() > 12
                        && name.len() <= 256
                        && !name.chars().any(char::is_control)
                })
                .ok_or(Error::InvalidHandle)?;
            if handles.iter().any(|previous| previous == handle) {
                continue;
            }
            if handles.len() >= 128 {
                return Err(Error::Capacity);
            }
            handles.try_reserve(1).map_err(|_| Error::Allocation)?;
            handles.push(handle.to_owned());
        }
        Ok(Self {
            unit,
            execution,
            handles,
            attempt: None,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
    pub fn pending_count(&self) -> usize {
        self.handles.len()
    }
    /// Register attempts only after successful encoding. Callers supply a fresh ID per attempt; failed receipts cannot overwrite later attempts.
    pub fn begin_release(&mut self, release_id: UnitId) -> Result<String, Error> {
        if self.attempt.is_some() {
            return Err(Error::Busy);
        }
        if self.is_empty() {
            return Err(Error::Empty);
        }
        let handles: Vec<&str> = self.handles.iter().map(String::as_str).collect();
        let message =
            loom_art_control::release_message(&self.unit, &self.execution, &release_id, &handles)
                .map_err(|_| Error::Encoding)?;
        self.attempt = Some(release_id);
        Ok(message)
    }
    /// Only the current attempt's success receipt clears leases. None means transport failure and permits another attempt, not fabricated success.
    pub fn complete_release(&mut self, release_id: &UnitId, outcome: Option<Outcome>) -> bool {
        if self.attempt.as_ref() != Some(release_id) {
            return false;
        }
        self.attempt = None;
        if outcome == Some(Outcome::Succeeded) {
            self.handles.clear();
        }
        true
    }
}
// Do not block on networking or spawn threads in Drop. Execution owners must explicitly handle pending_count before exit;
// retain failed leases in a bounded retry owner. This type alone does not guarantee remote release occurred.

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_output_leases_tests.rs"]
mod tests;
