//! Staged adoption retains original Editor, including redo branches; the first actual edit settles into one history step.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub enum AdoptionOutcome {
    Pending,
    Cancelled,
    Committed,
    Superseded,
}

/// Hosts call finish after all drafts/asynchronous edits end; do not discard this owner while unsettled.
pub struct Adoption {
    original: Option<Box<Editor>>,
    identity: std::sync::Arc<()>,
    revision: u64,
}

impl Editor {
    pub fn begin_adoption(&mut self, source: &Editor) -> Result<Option<Adoption>, EditError> {
        if self.draft.is_some() || source.draft.is_some() {
            return Ok(None);
        }
        let points = source
            .elements
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>();
        if source.elements.len() > MAX_ELEMENTS
            || self
                .retained_points
                .checked_add(points)
                .is_none_or(|total| total > MAX_TOTAL_POINTS)
        {
            return Err(EditError::BudgetExceeded);
        }
        let mut candidate = source.export_snapshot();
        candidate.style = self.style;
        // Original documents still consume point capacity; temporary edits must count too, without bypassing global document budgets.
        candidate.retained_points += self.retained_points;
        let identity = candidate.identity.clone();
        let revision = candidate.revision;
        let original = Box::new(std::mem::replace(self, candidate));
        Ok(Some(Adoption {
            original: Some(original),
            identity,
            revision,
        }))
    }
}

impl Adoption {
    pub fn unchanged(&self, editor: &Editor) -> bool {
        self.original.is_some()
            && std::sync::Arc::ptr_eq(&self.identity, &editor.identity)
            && self.revision == editor.revision
    }
    pub fn finish(&mut self, editor: &mut Editor) -> Result<AdoptionOutcome, EditError> {
        if self.original.is_none() || !std::sync::Arc::ptr_eq(&self.identity, &editor.identity) {
            self.original.take();
            return Ok(AdoptionOutcome::Superseded);
        }
        if editor.draft.is_some() {
            return Ok(AdoptionOutcome::Pending);
        }
        let original = self.original.as_mut().unwrap();
        let outcome = if editor.revision == self.revision {
            AdoptionOutcome::Cancelled
        } else {
            // Check budgets before modifying original; failures retain both states/owner for host handling.
            if !original.adopt_snapshot(original.revision(), editor)? {
                return Ok(AdoptionOutcome::Pending);
            }
            AdoptionOutcome::Committed
        };
        original.style = editor.style;
        *editor = *self.original.take().unwrap();
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "editor_adoption_tests.rs"]
mod tests;
