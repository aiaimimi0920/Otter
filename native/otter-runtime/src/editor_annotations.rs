//! Annotation-only replacement: freeze provenance during preparation and validate document identity on commit; original Editor always owns base images, crops, and logical frames.
use super::*;
use crate::edit_history::{Annotations, Entry};

/// Bounded one-shot candidates hold no source raster/history and cannot apply to different documents merely sharing revisions.
pub struct AnnotationReplacement {
    expected: DocumentStamp,
    annotations: Annotations,
    points: usize,
}
impl Editor {
    pub fn prepare_annotation_replacement(
        &self,
        source: &Editor,
    ) -> Result<AnnotationReplacement, EditError> {
        if self.draft.is_some() || source.draft.is_some() {
            return Err(EditError::InvalidPoint);
        }
        let points = source.elements.iter().map(|s| s.points.len()).sum();
        self.check_annotation_replacement(source.elements.len(), points)?;
        Ok(AnnotationReplacement {
            expected: self.document_stamp(),
            annotations: Annotations {
                elements: source.elements.clone(),
                serial: source.serial_count,
            },
            points,
        })
    }
    fn check_annotation_replacement(&self, count: usize, points: usize) -> Result<(), EditError> {
        if count > MAX_ELEMENTS
            || self
                .retained_points
                .checked_add(points)
                .is_none_or(|n| n > MAX_TOTAL_POINTS)
        {
            return Err(EditError::BudgetExceeded);
        }
        Ok(())
    }
}
impl AnnotationReplacement {
    /// Hosts must first validate their workspace/window and cancellation fences; this layer owns only Editor identity/committed-document transactions.
    pub fn apply(self, target: &mut Editor) -> Result<bool, EditError> {
        self.apply_rebased(target, None)
    }
    /// On initial collaboration-basis freezing, publish vectors/prepared bases atomically; every recoverable rejection precedes swaps.
    pub fn apply_rebased(
        self,
        target: &mut Editor,
        base: Option<crate::pixels::Pixels>,
    ) -> Result<bool, EditError> {
        if !self.expected.matches(target) || target.draft.is_some() {
            return Ok(false);
        }
        target.check_annotation_replacement(self.annotations.elements.len(), self.points)?;
        if base.as_ref().is_some_and(|base| {
            !target.source_replaceable()
                || target
                    .layers()
                    .is_none_or(|layers| layers.base.len() != base.len())
                || self.annotations.elements.iter().any(|s| s.blur.is_some())
        }) {
            return Ok(false);
        }
        // Only ownership transfer follows, with all recoverable validation complete; original history/tool styles do not adopt source state.
        let previous = Annotations {
            elements: std::mem::replace(&mut target.elements, self.annotations.elements),
            serial: std::mem::replace(&mut target.serial_count, self.annotations.serial),
        };
        target.retained_points += self.points;
        target.retained_points -= target.history.record(Entry::annotations(previous));
        if let Some(base) = base {
            target.history.rebase_source(&base);
            target.raster.as_mut().unwrap().base = base;
        }
        target.revision = target.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
#[path = "editor_annotations_tests.rs"]
mod tests;
