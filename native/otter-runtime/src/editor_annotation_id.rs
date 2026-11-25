//! Annotation identity moves with objects/history and survives previews/snapshots; user duplication must generate new identity.
use super::*;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AnnotationId(Arc<str>, Option<(f64, usize)>);

impl AnnotationId {
    pub(super) fn new() -> Result<Self, EditError> {
        let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }
            .map_err(|_| EditError::IdentityUnavailable)?;
        let d = guid.data4;
        Ok(Self(
            format!(
                "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                guid.data1, guid.data2, guid.data3, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]
            )
            .into(),
            None,
        ))
    }
    fn restored(value: &str) -> Result<Self, EditError> {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(EditError::InvalidPoint);
        }
        Ok(Self(value.into(), None))
    }
}

impl Stroke {
    pub fn annotation_id(&self) -> &str {
        &self.annotation_id.0
    }
    pub fn annotation_layer(&self) -> Option<(f64, usize)> {
        self.annotation_id.1
    }
}

impl Editor {
    pub fn with_restored_annotation_layers(mut self, layers: &[f64]) -> Result<Self, EditError> {
        if self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
            || layers.len() != self.elements.len()
            || layers
                .iter()
                .any(|z| !z.is_finite() || z.abs() > 1_000_000.0)
        {
            return Err(EditError::InvalidPoint);
        }
        for (rank, (stroke, &z)) in self.elements.iter_mut().zip(layers).enumerate() {
            stroke.annotation_id.1 = Some((z, rank));
        }
        Ok(self)
    }
    /// Install persistent identities only in unpublished restored documents, once after full validation, never rebinding by active indices.
    pub fn with_restored_annotation_ids(mut self, ids: &[String]) -> Result<Self, EditError> {
        if self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
            || ids.len() != self.elements.len()
        {
            return Err(EditError::InvalidPoint);
        }
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        let prepared = ids
            .iter()
            .map(|id| {
                if !seen.insert(id.as_str()) {
                    return Err(EditError::InvalidPoint);
                }
                AnnotationId::restored(id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (stroke, id) in self.elements.iter_mut().zip(prepared) {
            stroke.annotation_id = id;
        }
        Ok(self)
    }
}

#[cfg(test)]
#[path = "editor_annotation_id_tests.rs"]
mod tests;
