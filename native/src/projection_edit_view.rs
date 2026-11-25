//! Collaborative views preserve authoritative basis; local scaling is not user editing and ordinary sessions cannot renumber/crop wire data.
use super::{objects, Basis, Change, Document, Objects};
use serde_json::Value;
use std::{collections::BTreeSet, sync::Arc};
#[path = "projection_edit_view_mapping.rs"]
mod mapping;
#[path = "projection_edit_view_merge.rs"]
mod merge;
pub use merge::{reconcile, Conflict, Reconciliation};

// Internally derived coordinates may only increase JSON numeric digit counts; network/journal/per-object budgets remain original v1 gates.
const MAX_LOCAL_BYTES: usize = super::MAX_BYTES * 4;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    width: f64,
    height: f64,
}
impl Size {
    pub fn new(width: f64, height: f64) -> Result<Self, Error> {
        if [width, height]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0 || *v / 8192.0 == 0.0 || *v > 1_000_000.0)
        {
            return Err(Error::Size);
        }
        Ok(Self { width, height })
    }
    pub fn width(self) -> f64 {
        self.width
    }
    pub fn height(self) -> f64 {
        self.height
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Size,
    Limit,
    Annotation,
    Protocol(super::Error),
}
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    basis: Basis,
    size: Size,
    // resize shares only this bounded immutable basis tree, without compounding mean-axis scales or copying pixels/history.
    objects: Arc<Objects>,
}
impl View {
    pub fn from_basis(basis: &Basis, size: Size, objects: Objects) -> Result<Self, Error> {
        if !basis.valid() {
            return Err(Error::Protocol(super::Error::Basis));
        }
        bounded(&objects, super::MAX_BYTES)?;
        objects::validate(&objects).map_err(Error::Protocol)?;
        // Add default font sizes only during local derivation; added fields must not reject valid original objects at the 16KiB boundary.
        Ok(Self {
            basis: basis.clone(),
            size,
            objects: Arc::new(objects),
        })
    }
    pub fn from_document(document: &Document, size: Size) -> Result<Self, Error> {
        document.validate().map_err(Error::Protocol)?;
        Self::from_basis(document.basis(), size, document.live_objects())
    }
    pub fn basis(&self) -> &Basis {
        &self.basis
    }
    pub fn size(&self) -> Size {
        self.size
    }
    pub fn basis_objects(&self) -> &Objects {
        &self.objects
    }
    /// Output only wire anchors; platform Editor adapters handle text's 4px inset/font baseline and serial circular frames.
    pub fn local_objects(&self) -> Result<Objects, Error> {
        let result = mapping::forward(&self.objects, &self.basis, self.size)?;
        bounded(&result, MAX_LOCAL_BYTES)?;
        Ok(result)
    }
    pub fn resized(&self, size: Size) -> Self {
        Self {
            size,
            ..self.clone()
        }
    }
    /// Inverse-map all local candidates and pass original v1 gates first; failure leaves current view unchanged without partial publication.
    pub fn edited(&self, mut local: Objects) -> Result<Self, Error> {
        bounded(&local, MAX_LOCAL_BYTES)?;
        mapping::font_defaults(&mut local);
        let expected = self.local_objects()?;
        let objects = mapping::inverse(&local, &self.objects, &expected, &self.basis, self.size)?;
        Self::from_basis(&self.basis, self.size, objects)
    }
    /// Sort stable IDs and emit null for deletion; never regenerate zIndex from array positions.
    pub fn changes_to(&self, current: &Self) -> Result<Vec<Change>, Error> {
        self.same_basis(current)?;
        // Web tolerance applies in current local space; basis-space rounding must not discard small edits visible at high scale.
        let expected = self.resized(current.size).local_objects()?;
        let local = current.local_objects()?;
        let ids: BTreeSet<_> = expected.keys().chain(local.keys()).collect();
        Ok(ids
            .into_iter()
            .filter(|id| !equal_optional(expected.get(*id), local.get(*id)))
            .map(|id| Change {
                object_id: id.clone(),
                value: current.objects.get(id).cloned().unwrap_or(Value::Null),
            })
            .collect())
    }
    pub(super) fn same_basis(&self, other: &Self) -> Result<(), Error> {
        if self.basis != other.basis {
            return Err(Error::Protocol(super::Error::Basis));
        }
        Ok(())
    }
    pub(crate) fn overlay(&self, changes: &[Change]) -> Result<Self, Error> {
        validate_changes(changes)?;
        let mut objects = (*self.objects).clone();
        for change in changes {
            if change.value.is_null() {
                objects.remove(&change.object_id);
            } else {
                objects.insert(change.object_id.clone(), change.value.clone());
            }
        }
        Self::from_basis(&self.basis, self.size, objects)
    }
}

fn validate_changes(changes: &[Change]) -> Result<(), Error> {
    // Two 256-object views may yield 256 deletions + 256 additions; sending must still use original 32-entry batches.
    if changes.len() > 512 {
        return Err(Error::Limit);
    }
    if changes.iter().any(|c| !safe_tree(&c.value, 8)) {
        return Err(Error::Annotation);
    }
    serialized_bound(&changes, MAX_LOCAL_BYTES)?;
    let mut seen = BTreeSet::new();
    for change in changes {
        if !super::identifier(&change.object_id) || !seen.insert(&change.object_id) {
            return Err(Error::Protocol(super::Error::Identity));
        }
        if !change.value.is_null() && !objects::annotation(&change.object_id, &change.value) {
            return Err(Error::Protocol(super::Error::Annotation));
        }
    }
    Ok(())
}

// Same 1e-6 rounding boundary as Web editEqual; no second normalized tree, and array order remains meaningful.
fn edit_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let round = |v: &serde_json::Number| {
                let scaled = v.as_f64().unwrap() * 1e6;
                let floor = scaled.floor();
                // Do not use floor(n+0.5): addition near 2^52 itself rounds and wrongly merges adjacent valid coordinates.
                (if scaled - floor >= 0.5 {
                    floor + 1.0
                } else {
                    floor
                }) / 1e6
            };
            round(a) == round(b)
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| edit_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| edit_equal(a, b)))
        }
        _ => a == b,
    }
}
fn equal_optional(a: Option<&Value>, b: Option<&Value>) -> bool {
    edit_equal(a.unwrap_or(&Value::Null), b.unwrap_or(&Value::Null))
}

fn bounded(objects: &Objects, limit: usize) -> Result<(), Error> {
    // Reject deeply nested unknown extensions first so untrusted nesting cannot control general Value serialization/copy recursion.
    if objects.len() > 256 {
        return Err(Error::Limit);
    }
    if objects.values().any(|v| !safe_tree(v, 8)) {
        return Err(Error::Annotation);
    }
    serialized_bound(objects, limit)
}
fn safe_tree(value: &Value, left: u8) -> bool {
    if left == 0 {
        return false;
    }
    match value {
        Value::Array(v) => v.len() <= 2048 && v.iter().all(|v| safe_tree(v, left - 1)),
        Value::Object(v) => v.len() <= 256 && v.values().all(|v| safe_tree(v, left - 1)),
        _ => true,
    }
}
fn serialized_bound(value: &impl serde::Serialize, limit: usize) -> Result<(), Error> {
    struct Count {
        bytes: usize,
        limit: usize,
    }
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .filter(|v| *v <= self.limit)
                .ok_or_else(|| std::io::Error::other("edit view budget"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(&mut Count { bytes: 0, limit }, value).map_err(|_| Error::Limit)
}

#[cfg(all(test, windows))]
#[path = "projection_edit_view_journal_tests.rs"]
mod journal_tests;
#[cfg(test)]
#[path = "projection_edit_view_merge_tests.rs"]
mod merge_tests;
#[cfg(test)]
#[path = "projection_edit_view_test_fixture.rs"]
mod test_fixture;
#[cfg(test)]
#[path = "projection_edit_view_tests.rs"]
mod tests;
