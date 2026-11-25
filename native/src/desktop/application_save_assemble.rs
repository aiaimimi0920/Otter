//! Freeze a workspace into a complete session candidate; the caller supplies content encoding, and geometry-only candidates must never commit directly to disk.
use super::{Data, Document, FrozenData};
use crate::{
    session_write::{self, Candidate, Error, WindowCandidate},
    workspace_session::{SessionGroup, SessionLink, Sticker, Topology},
};
use serde_json::{Map, Value};
use std::sync::atomic::AtomicBool;

impl Data {
    /// Background complete-content candidate entry point; still requires UI Receipt revalidation and a disk-transaction commit, not direct publication.
    pub fn encode(mut self, cancel: &AtomicBool) -> Result<Candidate, Error> {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let _apartment = crate::desktop::Apartment::new().map_err(|_| Error::Window)?;
        for live in std::mem::take(&mut self.live) {
            self.documents.push(live.materialize(cancel)?);
        }
        let space = self.space;
        if self
            .transient_host
            .as_ref()
            .is_some_and(|host| self.projection_edits.iter().any(|(unit, _)| unit == host))
        {
            self.transient_host = None;
        }
        let mut projection_edits = std::mem::take(&mut self.projection_edits);
        self.assemble(cancel, |doc, fields| {
            let projection_edit = projection_edits
                .iter()
                .position(|(unit, _)| *unit == doc.unit)
                .map(|index| {
                    projection_edits
                        .swap_remove(index)
                        .1
                        .for_save(doc.annotation_mapping(&fields, space)?)
                        .map_err(|_| Error::Window)
                })
                .transpose()?;
            let mut candidate = encode_content(doc, fields, space, cancel)?;
            candidate.write_projection_edit(projection_edit.as_ref())?;
            Ok(candidate)
        })
    }
    /// Skip only an initial host that remains empty. Editing, pinning, grouping, linking, or selecting Art makes it a required save object.
    fn omit_host(&self, doc: &Document) -> bool {
        self.transient_host.as_ref() == Some(&doc.unit)
            && !self
                .port_visibility
                .iter()
                .any(|(unit, _)| unit == &doc.unit)
            && !doc.pinned
            && !doc.art
            && doc.group.is_none()
            && doc.parameters.is_none()
            && doc
                .sticker
                .as_ref()
                .is_none_or(|(config, accept)| !config.configured() && *accept)
            && doc.execution.is_none()
            && doc
                .node_settings
                .is_none_or(|settings| settings == Default::default())
            && doc.outputs.is_none()
            && doc.output_images.is_empty()
            && doc.editor.revision() == 0
            && doc.editor.elements().is_empty()
            && doc.editor.layers().is_none()
            && doc.editor.crop_view().is_none()
            && doc.restored_source.is_none()
            && doc.local_image.is_none()
            && doc.capture_meta.is_none()
            && doc.capture_original.is_none()
            && !self
                .projection_edits
                .iter()
                .any(|(unit, _)| *unit == doc.unit)
            && !self
                .links
                .iter()
                .any(|link| link.from_unit == doc.unit || link.to_unit == doc.unit)
    }

    /// content must encode images, cropping, and annotations; it consumes snapshots and must neither access active HWNDs nor publish side effects.
    pub fn assemble(
        self,
        cancel: &AtomicBool,
        mut content: impl FnMut(Document, Map<String, Value>) -> Result<WindowCandidate, Error>,
    ) -> Result<Candidate, Error> {
        if !self.live.is_empty() {
            return Err(Error::Window);
        }
        let topology = Topology {
            stickers: self
                .documents
                .iter()
                .filter(|doc| !self.omit_host(doc))
                .map(|doc| Sticker {
                    id: doc.unit.as_str().into(),
                    group_id: doc.group.as_ref().map(|id| id.as_str().into()),
                })
                .collect(),
            links: self
                .links
                .iter()
                .map(|link| SessionLink {
                    id: link.id.as_str().into(),
                    from_unit_id: link.from_unit.as_str().into(),
                    from_port_id: link.from_port.clone(),
                    to_unit_id: link.to_unit.as_str().into(),
                    to_port_id: link.to_port.clone(),
                })
                .collect(),
            groups: self
                .groups
                .iter()
                .map(|group| SessionGroup {
                    id: group.id.as_str().into(),
                    name: group.name.clone(),
                    hidden: group.hidden,
                    locked: group.locked,
                })
                .collect(),
        };
        // Validate the original list first so HashMap overwrite semantics cannot hide duplicate window identities.
        let mut ids = std::collections::HashSet::new();
        if self
            .documents
            .iter()
            .any(|doc| !ids.insert(doc.unit.clone()))
        {
            return Err(Error::Topology);
        }
        let mut documents: std::collections::HashMap<_, _> = self
            .documents
            .into_iter()
            .map(|doc| (doc.unit.as_str().to_owned(), doc))
            .collect();
        session_write::assemble_owned_at_revision(
            &self.original,
            &topology,
            cancel,
            Some(self.space),
            self.disk_revision,
            Some(self.projection_scope),
            |sticker, original| {
                let mut doc = documents.remove(&sticker.id).ok_or(Error::Topology)?;
                let fields = merge_document(&mut doc, original, self.space, &self.port_visibility)?;
                content(doc, fields)
            },
        )
    }
}

impl FrozenData {
    pub(super) fn original(&self) -> &crate::session_source::Source {
        &self.0.original
    }
    pub(super) fn encode(
        self,
        source: &crate::session_document::Document<'_>,
        unit: &crate::workspace_id::UnitId,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels, Error> {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let mut data = self.0;
        if data.documents.len() + data.live.len() != 1 {
            return Err(Error::Window);
        }
        let _apartment = crate::desktop::Apartment::new().map_err(|_| Error::Window)?;
        let mut doc = if let Some(live) = data.live.pop() {
            live.materialize(cancel)?
        } else {
            let doc = data.documents.pop().expect("one captured window");
            if data.omit_host(&doc) {
                return Err(Error::Window);
            }
            doc
        };
        if &doc.unit != unit || doc.art {
            return Err(Error::Window);
        }
        let mut old = previous_window(source, unit.as_str())?;
        // Graph outputs do not belong to a frozen sticker. Do not encode them only to discard them later.
        old.remove("outputs");
        doc.outputs = None;
        doc.output_images.clear();
        let mut fields = merge_document(&mut doc, old, data.space, &[])?;
        fields.insert("id".into(), unit.as_str().into());
        let candidate = encode_content(doc, fields, data.space, cancel)?;
        session_write::encode_bounded(&candidate, cancel)
    }
}

fn previous_window(
    source: &crate::session_document::Document<'_>,
    id: &str,
) -> Result<Map<String, Value>, Error> {
    let Some(index) = source.windows.iter().position(|window| window.id == id) else {
        return Ok(Map::new());
    };
    #[derive(serde::Deserialize)]
    struct RawWindows<'a> {
        #[serde(borrow)]
        stickers: Vec<&'a serde_json::value::RawValue>,
    }
    // The source has already validated the same array. Only materialize the selected window.
    let raw: RawWindows<'_> =
        serde_json::from_slice(source.original_json()).map_err(|_| Error::Source)?;
    serde_json::from_str(raw.stickers[index].get()).map_err(|_| Error::Source)
}

fn merge_document(
    doc: &mut Document,
    original: Map<String, Value>,
    space: crate::session_placement::Space,
    port_visibility: &[(
        crate::workspace_id::UnitId,
        crate::workspace::ports::Visibility,
    )],
) -> Result<Map<String, Value>, Error> {
    let geometry = doc.geometry().map_err(|_| Error::Window)?;
    let Value::Object(mut fields) =
        crate::session_window_write::merge(original.into(), geometry, space)
            .map_err(|_| Error::Window)?
    else {
        return Err(Error::Window);
    };
    // Without the current session owner, do not carry old snapshots unchanged from source content and revive them after deletion/reset.
    fields.remove(crate::projection_edit::workspace::FIELD);
    if let Some(extensions) = &doc.extensions {
        extensions.write(&mut fields).map_err(|_| Error::Window)?;
    }
    if let Some((_, visibility)) = port_visibility.iter().find(|(unit, _)| unit == &doc.unit) {
        fields.insert(
            "portVisibility".into(),
            serde_json::to_value(visibility).map_err(|_| Error::Window)?,
        );
    } else {
        fields.remove("portVisibility");
    }
    if let Some(outputs) = doc.outputs.take() {
        fields.insert("outputs".into(), Value::Object(outputs));
    }
    if doc.capture_original.is_some()
        || doc
            .capture_meta
            .as_ref()
            .and_then(|metadata| metadata.get("dynamicRange"))
            .and_then(Value::as_str)
            != Some("hdr")
    {
        fields.remove("otterNativeCaptureOriginalSrc");
    }
    if let Some(metadata) = doc.capture_meta.take() {
        let raw = serde_json::to_string(&metadata).map_err(|_| Error::Window)?;
        crate::session_capture::decode(&raw).map_err(|_| Error::Window)?;
        fields.insert("captureMeta".into(), metadata);
    } else {
        fields.remove("captureMeta");
    }
    let previous_crop = fields.remove("imageEditState");
    let crop = if doc.restored_source.is_some() {
        doc.editor.crop_view()
    } else {
        doc.editor.raster_crop_view()
    };
    // Only the selected PNG owns this marker. A reset changes the view, not
    // whether the underlying texture already includes the full-frame margins.
    let rasterized = crop.is_none_or(|view| view.legacy_pixel_region().is_none())
        && if doc.restored_source.is_some() {
            doc.restored_source_rasterized_crop
        } else {
            doc.editor.layers().is_some_and(|layers| {
                doc.editor.full_logical_frame().is_some_and(|frame| {
                    crate::image_projection::Projection::raster_density(layers.size, frame)
                        .is_some()
                })
            })
        };
    if previous_crop.is_some()
        || crop.is_some()
        || rasterized
        || doc.frame_style.is_some_and(|s| s != Default::default())
        || doc.editor.mirror_state() != Default::default()
    {
        let mut state = crate::session_crop_write::merge_view(previous_crop, crop)
            .map_err(|_| Error::Window)?;
        let edit_fields = state.as_object_mut().ok_or(Error::Window)?;
        for (name, active) in [
            (
                crate::session_crop::RASTERIZED_SOURCE_FIELD,
                rasterized && crop.is_some(),
            ),
            (
                crate::session_crop::FULL_RASTERIZED_SOURCE_FIELD,
                rasterized && crop.is_none(),
            ),
        ] {
            if active {
                edit_fields.insert(name.into(), true.into());
            } else {
                edit_fields.remove(name);
            }
        }
        if let Some(style) = doc.frame_style {
            state = style.merge(state).map_err(|_| Error::Window)?;
        }
        doc.editor
            .mirror_state()
            .write(&mut state)
            .map_err(|_| Error::Window)?;
        fields.insert("imageEditState".into(), state);
    }
    match (&doc.parameters, doc.execution) {
        (Some(parameters), Some(execution)) if doc.art => {
            fields.remove("otterNativeLocalArt");
            fields.remove("otterNativeLocalArtPorts");
            fields = crate::session_art_write::merge(fields, parameters, execution);
        }
        (None, None) => {
            if let Some(id) = &doc.local_art_id {
                if !doc.art || id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
                    return Err(Error::Window);
                }
                fields.insert("type".into(), "art".into());
                fields.insert("artId".into(), id.clone().into());
                fields.insert("otterNativeLocalArt".into(), true.into());
                if let Some(ports) = &doc.local_art_ports {
                    ports.validate(id).map_err(|_| Error::Window)?;
                    fields.insert(
                        "otterNativeLocalArtPorts".into(),
                        serde_json::to_value(ports).map_err(|_| Error::Window)?,
                    );
                } else {
                    fields.remove("otterNativeLocalArtPorts");
                }
            }
            if doc.art && fields.get("type").and_then(Value::as_str) != Some("art") {
                return Err(Error::Window);
            }
            if !doc.art {
                fields.remove("otterNativeLocalArt");
                fields.remove("otterNativeLocalArtPorts");
                fields.remove("artId");
                fields.insert("type".into(), "sticker".into());
                let (config, accept) = doc.sticker.clone().unwrap_or_default();
                let params = config.parameters();
                crate::sticker_image_params::validate(&params).map_err(|_| Error::Window)?;
                fields.insert("params".into(), Value::Object(params));
                if doc
                    .extensions
                    .as_ref()
                    .is_none_or(|e| e.has_source_revision())
                {
                    let propagation = fields
                        .entry("stickerEditPropagation")
                        .or_insert_with(|| Value::Object(Map::new()));
                    let propagation = propagation.as_object_mut().ok_or(Error::Window)?;
                    propagation.insert(
                        "acceptUpstream".into(),
                        if doc.sticker.is_some() { accept } else { true }.into(),
                    );
                    propagation.insert("locallyEdited".into(), doc.editor.has_edit_state().into());
                }
            }
        }
        _ => return Err(Error::Window),
    }
    if let Some(settings) = doc.node_settings {
        settings
            .write(&mut fields, doc.art)
            .map_err(|_| Error::Window)?;
    }
    Ok(fields)
}

fn encode_content(
    doc: Document,
    fields: Map<String, Value>,
    space: crate::session_placement::Space,
    cancel: &AtomicBool,
) -> Result<WindowCandidate, Error> {
    let fields = doc.write_annotations(fields, space, cancel)?;
    let original = doc
        .capture_original
        .as_ref()
        .map(|file| crate::capture_file::encode_inline(file, cancel))
        .transpose()
        .map_err(|error| {
            if error == crate::loom_base64::Error::Cancelled {
                Error::Cancelled
            } else {
                Error::Window
            }
        })?;
    let images = doc.encode_images(cancel).map_err(|_| {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            Error::Cancelled
        } else {
            Error::Window
        }
    })?;
    let mut candidate = match images {
        Some(images) => images
            .into_data_urls(cancel)
            .map_err(|_| {
                if cancel.load(std::sync::atomic::Ordering::Acquire) {
                    Error::Cancelled
                } else {
                    Error::Window
                }
            })?
            .install(fields),
        None => Ok(WindowCandidate::new(fields)),
    }?;
    if let Some(original) = original {
        candidate.insert_encoded("otterNativeCaptureOriginalSrc", original);
    }
    for (name, image) in doc.output_images {
        let png = image.png(cancel).map_err(|_| {
            if cancel.load(std::sync::atomic::Ordering::Acquire) {
                Error::Cancelled
            } else {
                Error::Window
            }
        })?;
        let encoded = crate::loom_base64::encode_png_data_url(&png, cancel).map_err(|error| {
            if error == crate::loom_base64::Error::Cancelled {
                Error::Cancelled
            } else {
                Error::Budget
            }
        })?;
        candidate.insert_encoded_output(&name, encoded)?;
    }
    Ok(candidate)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_save_output_image_tests.rs"]
mod output_image_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_save_ports_tests.rs"]
mod ports_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_save_roundtrip_tests.rs"]
mod roundtrip_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_save_sticker_tests.rs"]
mod sticker_tests;
#[cfg(test)]
#[path = "application_save_assemble_tests.rs"]
mod tests;
