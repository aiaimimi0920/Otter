//! Execution-request owner: one current request per object, with generation matching blocking stale delivery; owns no threads or network connections.
use crate::{workspace::MAX_UNITS, workspace_id::UnitId};
pub const MAX_PREVIEW_BYTES: usize = 65536;
pub const MAX_TOTAL_PREVIEW_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: UnitId,
    pub generation: u64,
}
struct Entry {
    unit: UnitId,
    generation: u64,
    active: Option<Request>,
    preview: Option<String>,
}
impl Entry {
    fn retire(&mut self) -> usize {
        self.active = None;
        self.preview.take().map_or(0, |preview| preview.len())
    }
}
#[derive(Default)]
pub struct Requests {
    entries: Vec<Entry>,
    preview_bytes: usize,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Capacity,
    Allocation,
    GenerationExhausted,
    PreviewTooLarge,
}
impl Requests {
    /// Retained historical generations are not busy; an active request on any node prevents whole-workspace replacement.
    pub fn is_idle(&self) -> bool {
        self.entries.iter().all(|entry| entry.active.is_none())
    }
    pub fn next_generation(&self, unit: &UnitId) -> Result<u64, Error> {
        match self.entries.iter().find(|entry| &entry.unit == unit) {
            Some(entry) => entry
                .generation
                .checked_add(1)
                .ok_or(Error::GenerationExhausted),
            None if self.entries.len() >= MAX_UNITS => Err(Error::Capacity),
            None => Ok(1),
        }
    }
    /// Callers provide a new request UUID; even if an ID is mistakenly reused, delivery must match the full generation token.
    pub fn begin(&mut self, unit: UnitId, id: UnitId) -> Result<Request, Error> {
        let index = match self.entries.iter().position(|entry| entry.unit == unit) {
            Some(index) => index,
            None => {
                if self.entries.len() >= MAX_UNITS {
                    return Err(Error::Capacity);
                }
                self.entries.try_reserve(1).map_err(|_| Error::Allocation)?;
                self.entries.push(Entry {
                    unit,
                    generation: 0,
                    active: None,
                    preview: None,
                });
                self.entries.len() - 1
            }
        };
        let entry = &mut self.entries[index];
        let generation = entry
            .generation
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)?;
        let request = Request { id, generation };
        entry.generation = generation;
        entry.active = Some(request.clone());
        if let Some(preview) = entry.preview.take() {
            self.preview_bytes -= preview.len();
        }
        Ok(request)
    }
    pub fn active(&self, unit: &UnitId) -> Option<&Request> {
        self.entries
            .iter()
            .find(|entry| &entry.unit == unit)?
            .active
            .as_ref()
    }
    pub fn is_latest(&self, unit: &UnitId, request: &Request) -> bool {
        self.active(unit) == Some(request)
    }
    /// The protocol decoder already limits message bytes; take ownership of strings here without additionally copying image references.
    pub fn mark_preview(
        &mut self,
        unit: &UnitId,
        request: &Request,
        preview: String,
    ) -> Result<bool, Error> {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| &entry.unit == unit && entry.active.as_ref() == Some(request))
        else {
            return Ok(false);
        };
        let old_bytes = entry.preview.as_ref().map_or(0, String::len);
        let next_bytes = self.preview_bytes - old_bytes + preview.len();
        if preview.len() > MAX_PREVIEW_BYTES || next_bytes > MAX_TOTAL_PREVIEW_BYTES {
            return Err(Error::PreviewTooLarge);
        }
        entry.preview = Some(preview);
        self.preview_bytes = next_bytes;
        Ok(true)
    }
    pub fn preview(&self, unit: &UnitId, request: &Request) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| &entry.unit == unit && entry.active.as_ref() == Some(request))?
            .preview
            .as_deref()
    }
    pub fn finish(&mut self, unit: &UnitId, request: &Request) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| &entry.unit == unit && entry.active.as_ref() == Some(request))
        else {
            return false;
        };
        self.preview_bytes -= entry.retire();
        true
    }
    /// Cancellation preserves generation; the next begin increases it monotonically, so old callbacks cannot become current again.
    pub fn invalidate(&mut self, unit: &UnitId) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| &entry.unit == unit) {
            self.preview_bytes -= entry.retire();
        }
    }
    /// Remove metadata and release budget when an object closes; new objects in the same workspace must use new UnitIds.
    pub fn retain_units(&mut self, mut keep: impl FnMut(&UnitId) -> bool) {
        self.entries.retain(|entry| keep(&entry.unit));
        self.preview_bytes = self
            .entries
            .iter()
            .filter_map(|entry| entry.preview.as_ref())
            .map(String::len)
            .sum();
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.preview_bytes = 0;
    }
    pub fn preview_bytes(&self) -> usize {
        self.preview_bytes
    }
}
#[cfg(test)]
#[path = "graph_requests_tests.rs"]
mod tests;
