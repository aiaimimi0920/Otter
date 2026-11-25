//! Annotation adapter between View/existing Editor; basis/original fields remain authoritative, without another editor or raster writes.
use super::{view::View, Objects};
use crate::{
    editor::{AnnotationReplacement, DocumentStamp, Editor},
    session_vectors::{self, TextMetrics},
    text_annotation::TextAnnotation,
};
use serde_json::{json, Value};
#[path = "projection_edit_editor_fields.rs"]
mod fields;

#[derive(Debug, PartialEq)]
pub enum Error {
    View(super::view::Error),
    Read(session_vectors::Error),
    Write(crate::session_vector_write::Error),
    Edit(crate::editor::EditError),
    Identity,
    Unsupported,
    Limit,
}
pub struct Prepared {
    source: View,
    expected: Objects,
    replacement: AnnotationReplacement,
    mapping: Option<crate::session_vector_write::Mapping>,
}
/// expected is only an f32 Editor projection; source preserves unchanged wire f64/defaults/color lexemes/stable hierarchy.
pub struct Bridge {
    source: View,
    expected: Objects,
    document: DocumentStamp,
    mapping: Option<crate::session_vector_write::Mapping>,
}
pub fn prepare(
    view: &View,
    target: &Editor,
    measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
) -> Result<Prepared, Error> {
    prepare_in(view, target, None, measure)
}
/// Restored Editor may have canonical padding/scale; reuse original session mapping without mistaking position for basis scaling.
pub fn prepare_mapped(
    view: &View,
    target: &Editor,
    mapping: crate::session_vector_write::Mapping,
    measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
) -> Result<Prepared, Error> {
    prepare_in(view, target, Some(mapping), measure)
}
fn prepare_in(
    view: &View,
    target: &Editor,
    mapping: Option<crate::session_vector_write::Mapping>,
    mut measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
) -> Result<Prepared, Error> {
    let local = view.local_objects().map_err(Error::View)?;
    let raw = serde_json::value::to_raw_value(&json!({"serialCounter":target.serial_count(),"elements":local.values().collect::<Vec<_>>()})).map_err(|_| Error::Limit)?;
    let mut restored = session_vectors::restore_with_text(Some(&raw), &mut measure)
        .map_err(Error::Read)?
        .editor;
    if let Some(mapping) = mapping {
        let local = encoded(
            &restored,
            &mut measure,
            Some(mapping.inverse().map_err(Error::Write)?),
        )?;
        let raw = serde_json::value::to_raw_value(&json!({"serialCounter":target.serial_count(),"elements":local.values().collect::<Vec<_>>()}))
            .map_err(|_| Error::Limit)?;
        restored = session_vectors::restore_with_text(Some(&raw), &mut measure)
            .map_err(Error::Read)?
            .editor;
    }
    let expected = encoded(&restored, &mut measure, mapping)?;
    Ok(Prepared {
        source: view.clone(),
        expected,
        replacement: target
            .prepare_annotation_replacement(&restored)
            .map_err(Error::Edit)?,
        mapping,
    })
}
impl Prepared {
    /// Hosts also check workspace/window/cancellation; expiry/document changes produce no bridge or partial annotations.
    pub fn apply(self, target: &mut Editor) -> Result<Option<Bridge>, Error> {
        self.apply_rebased(target, None)
    }
    pub(crate) fn apply_rebased(
        self,
        target: &mut Editor,
        base: Option<crate::pixels::Pixels>,
    ) -> Result<Option<Bridge>, Error> {
        if !self
            .replacement
            .apply_rebased(target, base)
            .map_err(Error::Edit)?
        {
            return Ok(None);
        }
        Ok(Some(Bridge {
            source: self.source,
            expected: self.expected,
            document: target.document_stamp(),
            mapping: self.mapping,
        }))
    }
}
impl Bridge {
    /// The staged initial view predates networking; expected must come from that view's Editor projection, not the current Editor as an old baseline.
    pub(crate) fn from_staged(
        source: View,
        editor: &Editor,
        mapping: crate::session_vector_write::Mapping,
        measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    ) -> Result<Self, Error> {
        let prepared = prepare_mapped(&source, editor, mapping, measure)?;
        Ok(Self {
            source: prepared.source,
            expected: prepared.expected,
            document: editor.document_stamp(),
            mapping: prepared.mapping,
        })
    }
    /// Bind only after host validation of exact canonical contents/identity; no Editor rewriting or added recovery undo.
    pub fn from_checkpoint(
        source: View,
        editor: &Editor,
        mapping: crate::session_vector_write::Mapping,
        mut measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    ) -> Result<Self, Error> {
        if editor.draft().is_some() {
            return Err(Error::Identity);
        }
        let mut expected = encoded(editor, &mut measure, Some(mapping))?;
        // Editor objects never observed by source are local additions, not missing original values just because expected=current.
        expected.retain(|id, _| source.basis_objects().contains_key(id));
        Ok(Self {
            source,
            expected,
            document: editor.document_stamp(),
            mapping: Some(mapping),
        })
    }
    pub fn source(&self) -> &View {
        &self.source
    }
    /// After every asynchronous result, hosts reread current Editor; old snapshots must not overwrite edits/undo made during saving.
    pub fn current(
        &self,
        editor: &Editor,
        mut measure: impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    ) -> Result<View, Error> {
        if !self.document.same_document(editor) || editor.draft().is_some() {
            return Err(Error::Identity);
        }
        let current = encoded(editor, &mut measure, self.mapping)?;
        let original = self.source.local_objects().map_err(Error::View)?;
        // Share canonical hierarchy plans; assign new values in actual Editor order and renumber all on overflow.
        let order = crate::session_vector_write::OrderPlan::new(editor, original.values());
        let mut result = Objects::new();
        for (index, stroke) in editor.elements().iter().enumerate() {
            let id = stroke.annotation_id();
            let current = current.get(id).ok_or(Error::Identity)?;
            let mut value = fields::merge(original.get(id), self.expected.get(id), Some(current))
                .ok_or(Error::Unsupported)?;
            value["zIndex"] = order.value(id, index);
            fields::wire(&mut value)?;
            result.insert(id.to_owned(), value);
        }
        self.source.edited(result).map_err(Error::View)
    }
}
fn encoded(
    editor: &Editor,
    measure: &mut impl FnMut(&TextAnnotation) -> Result<TextMetrics, session_vectors::Error>,
    mapping: Option<crate::session_vector_write::Mapping>,
) -> Result<Objects, Error> {
    if editor.elements().len() > 256
        || editor.elements().iter().any(|s| {
            s.points.len() > 2048
                || s.text
                    .as_ref()
                    .is_some_and(|t| t.content().encode_utf16().count() > 4096)
        })
    {
        return Err(Error::Limit);
    }
    let mut baseline = |text: &TextAnnotation| {
        measure(text)
            .map(|v| v.baseline)
            .map_err(|_| crate::session_vector_write::Error::Geometry)
    };
    let bytes = match mapping {
        Some(mapping) => {
            crate::session_vector_write::encode_mapped(editor, None, mapping, &mut baseline)
        }
        None => crate::session_vector_write::encode(editor, None, &mut baseline),
    }
    .map_err(Error::Write)?;
    if bytes.len() > super::MAX_BYTES * 4 {
        return Err(Error::Limit);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| Error::Unsupported)?;
    value["elements"]
        .as_array()
        .ok_or(Error::Unsupported)?
        .iter()
        .map(|v| {
            Ok((
                v["id"].as_str().ok_or(Error::Unsupported)?.to_owned(),
                v.clone(),
            ))
        })
        .collect()
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_edit_editor_tests.rs"]
mod tests;
