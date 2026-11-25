//! Project active catalogs/document attachments into bounded Surface data, without commands/resource-access permits.
use crate::{
    loom_extension_snapshot::{text, Declaration, Snapshot},
    unit_attachment_store::{Store, Stored},
};
use serde_json::Value;
#[path = "extension_visual_availability.rs"]
pub mod availability;
#[path = "extension_visual_scene.rs"]
pub mod scene;
pub const OCR: &str = "neuro.official/ocr";
pub const OCR_RESULT: &str = "neuro.official/ocr.result.v1";
pub const TRANSLATE: &str = "neuro.official/text-translation";
pub const TRANSLATION: &str = "neuro.official/text-translation.result.v1";
#[derive(Clone, Copy)]
pub enum Bounds {
    Fixed([f32; 4]),
    UnitInset(f32),
}
impl Bounds {
    pub fn resolve(self, size: [f32; 2]) -> [f32; 4] {
        match self {
            Self::Fixed(bounds) => bounds,
            Self::UnitInset(inset) => [
                inset,
                inset,
                (size[0] - inset * 2.).max(0.),
                (size[1] - inset * 2.).max(0.),
            ],
        }
    }
}
pub struct Visual {
    pub plugin: String,
    pub attachment: String,
    pub descriptor: String,
    pub bounds: Bounds,
    pub scene: Value,
    pub command: Option<String>,
    pub local_copy: bool,
    pub generation: u64,
    pub revision: u64,
    pub placeholder: bool,
}
#[derive(Clone, Copy)]
struct Descriptor<'a> {
    id: &'a str,
    bounds: Bounds,
    command: Option<&'a str>,
    payload: &'a Value,
    source: bool,
    replaces: bool,
    generation: u64,
    fallback: bool,
}
fn descriptor<'a>(
    catalog: &'a Snapshot,
    item: Declaration<'a>,
    attachment: &Stored,
    renderer: bool,
) -> Option<Descriptor<'a>> {
    let a = &attachment.attachment;
    if item.plugin != a.plugin()
        || !catalog
            .visual_declarations("dataTypes")
            .any(|d| d.scope == item.scope && d.plugin == item.plugin && d.id == a.type_id())
        || renderer && a.renderer().is_some_and(|id| id != item.id)
    {
        return None;
    }
    let envelope = item.payload?;
    if envelope.get("schema")?.as_str()?
        != if renderer {
            "renderer.v1"
        } else {
            "unit-overlay.v1"
        }
    {
        return None;
    }
    let p = envelope.get("payload")?;
    if p.get("typeId")?.as_str()? != a.type_id() {
        return None;
    }
    let path = p.get("attachmentScenePath");
    if path.is_some_and(|v| !valid_path(v))
        || path.is_none() && scene::sanitize(p.get("scene")?, item.command, false, None).is_none()
    {
        return None;
    }
    let b = p.get("bounds")?;
    let mut bounds = [0.; 4];
    for (i, key) in ["x", "y", "width", "height"].iter().enumerate() {
        let n = b.get(key)?.as_f64()?;
        if !n.is_finite() || !(if i < 2 { 0. } else { 1. }..=8192.).contains(&n) {
            return None;
        }
        bounds[i] = n as f32;
    }
    if let Some(command) = item.command {
        if !catalog.visual_declarations("commands").any(|c| {
            c.plugin == item.plugin && c.scope == item.scope && c.command.unwrap_or(c.id) == command
        }) {
            return None;
        }
    }
    let source = match p.get("sourceContext") {
        None => false,
        Some(v) if v == "ocr-text.v1" => true,
        _ => return None,
    };
    let replaces = match p.get("replacesSource") {
        None => false,
        Some(v) if source => v.as_bool()?,
        _ => return None,
    };
    Some(Descriptor {
        id: item.id,
        bounds: Bounds::Fixed(bounds),
        command: item.command,
        payload: p,
        source,
        replaces,
        generation: catalog.generation(),
        fallback: false,
    })
}
pub fn source_current(store: &Store, item: &Stored, revision: u64) -> bool {
    let Some(source) = store.items().iter().find(|s| {
        s.attachment.id() == "neuro.official/ocr.result"
            && s.attachment.plugin() == OCR
            && s.attachment.type_id() == OCR_RESULT
            && s.attachment.schema() == "1"
    }) else {
        return false;
    };
    let Some(p) = item.attachment.payload() else {
        return false;
    };
    let Some(dependency) = p.get("sourceAttachment") else {
        return false;
    };
    p.get("sourceRevision").and_then(Value::as_u64) == Some(revision)
        && dependency.get("attachmentId").and_then(Value::as_str) == Some(source.attachment.id())
        && dependency.get("revision").and_then(Value::as_u64) == Some(source.attachment.revision())
        && dependency
            .get("digest")
            .and_then(Value::as_str)
            .is_some_and(|d| source.digest.as_deref() == Some(d))
        && p.get("originalText")
            .and_then(Value::as_str)
            .is_some_and(|t| {
                source
                    .attachment
                    .payload()
                    .and_then(|p| p.get("fullText"))
                    .and_then(Value::as_str)
                    == Some(t)
            })
}
fn prepare(d: Descriptor<'_>, attachment: &Stored) -> Option<Visual> {
    if d.fallback {
        return scene::sanitize(
            d.payload.get("surfaceScene")?,
            d.command,
            true,
            attachment.visible,
        )
        .map(|scene| visual(d, attachment, scene));
    }
    let attached = d.payload.get("attachmentScenePath").map(|path| {
        let path = path.as_str()?.strip_prefix('/')?;
        let segments: Vec<_> = path.split('/').collect();
        if segments.is_empty()
            || segments.len() > 8
            || segments.iter().any(|s| {
                s.is_empty()
                    || s.len() > 64
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            })
        {
            return None;
        }
        let mut value = attachment.attachment.payload()?;
        for segment in segments {
            value = value.as_object()?.get(segment)?;
        }
        scene::sanitize(value, d.command, true, attachment.visible)
    });
    let scene = attached.flatten().or_else(|| {
        scene::sanitize(
            d.payload.get("scene")?,
            d.command,
            false,
            attachment.visible,
        )
    })?;
    Some(visual(d, attachment, scene))
}
fn visual(d: Descriptor<'_>, attachment: &Stored, scene: Value) -> Visual {
    Visual {
        plugin: attachment.attachment.plugin().into(),
        attachment: attachment.attachment.id().into(),
        descriptor: d.id.into(),
        bounds: d.bounds,
        scene,
        command: d.command.map(str::to_owned),
        local_copy: attachment.attachment.schema() == "1"
            && matches!(
                (
                    attachment.attachment.plugin(),
                    attachment.attachment.type_id()
                ),
                (OCR, OCR_RESULT) | (TRANSLATE, TRANSLATION)
            ),
        generation: d.generation,
        revision: attachment.attachment.revision(),
        placeholder: false,
    }
}
fn valid_path(value: &Value) -> bool {
    value
        .as_str()
        .and_then(|s| s.strip_prefix('/'))
        .is_some_and(|path| {
            let parts: Vec<_> = path.split('/').collect();
            parts.len() <= 8
                && parts.iter().all(|s| {
                    text(s, 64)
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                })
        })
}
pub fn project(catalog: Option<&Snapshot>, store: &Store, revision: u64) -> Vec<Visual> {
    let mut visuals = catalog
        .map(|c| project_available(c, store, revision))
        .unwrap_or_default();
    if let Some(placeholder) = availability::unavailable(catalog, store, &visuals) {
        visuals.push(placeholder);
    }
    visuals
}
fn project_available(catalog: &Snapshot, store: &Store, revision: u64) -> Vec<Visual> {
    let mut candidates = Vec::new();
    for item in store.items().iter().filter(|i| i.is_visible()) {
        let renderer = catalog
            .visual_declarations("renderers")
            .take(32)
            .find_map(|d| descriptor(catalog, d, item, true))
            .or_else(|| availability::ocr(catalog, item));
        for d in renderer.into_iter().chain(
            catalog
                .visual_declarations("unitOverlays")
                .take(32)
                .filter_map(|d| descriptor(catalog, d, item, false)),
        ) {
            if d.source && !source_current(store, item, revision) {
                continue;
            }
            candidates.push((item, d));
        }
    }
    const MAX_VISUALS: usize = 32;
    let is_source = |item: &Stored| item.attachment.id() == "neuro.official/ocr.result";
    // Only valid scenes actually entering replacement projections may hide OCR; retain the first successfully prepared scene.
    let mut replacement = candidates
        .iter()
        .enumerate()
        .filter(|(_, (item, _))| !is_source(item))
        .take(MAX_VISUALS)
        .find_map(|(index, (item, d))| {
            (d.replaces && d.source)
                .then(|| prepare(*d, item).map(|v| (index, v)))
                .flatten()
        });
    let replaces = replacement.is_some();
    candidates
        .into_iter()
        .enumerate()
        .filter(|(_, (item, _))| !replaces || !is_source(item))
        .take(MAX_VISUALS)
        .map(|(index, (item, d))| {
            if replacement.as_ref().is_some_and(|(i, _)| *i == index) {
                return replacement.take().unwrap().1;
            }
            prepare(d, item).unwrap_or_else(|| visual(d, item, availability::scene_unavailable()))
        })
        .collect()
}
pub fn cached(catalog: &Snapshot, store: &Store, revision: u64, translation: bool) -> bool {
    let plugin = if translation { TRANSLATE } else { OCR };
    store.items().iter().any(|item| {
        let a = &item.attachment;
        a.plugin() == plugin
            && (if translation {
                a.type_id() == TRANSLATION
            } else {
                [OCR_RESULT, "neuro.official/ocr.codes.v1"].contains(&a.type_id())
            })
            && catalog
                .visual_declarations("dataTypes")
                .any(|d| d.plugin == plugin && d.id == a.type_id())
            && !catalog
                .visual_declarations("renderers")
                .take(32)
                .find_map(|d| descriptor(catalog, d, item, true))
                .is_some_and(|d| d.source && !source_current(store, item, revision))
    })
}
/// None means no usable cache; later command-execution owners must perform real initial authorization/recognition.
pub fn toggle(
    catalog: &Snapshot,
    store: &mut Store,
    revision: u64,
    translation: bool,
) -> Result<bool, crate::unit_attachment_store::Error> {
    let plugin = if translation { TRANSLATE } else { OCR };
    let mut ids = Vec::new();
    let mut replacements = Vec::new();
    for item in store.items() {
        let a = &item.attachment;
        let renderer = catalog
            .visual_declarations("renderers")
            .take(32)
            .find_map(|d| descriptor(catalog, d, item, true));
        if renderer.as_ref().is_some_and(|d| d.replaces && d.source)
            && item
                .attachment
                .payload()
                .and_then(|p| p.pointer("/sourceAttachment/attachmentId"))
                .and_then(Value::as_str)
                == Some("neuro.official/ocr.result")
        {
            replacements.push(a.id().to_owned());
        }
        if a.plugin() != plugin
            || !(if translation {
                a.type_id() == TRANSLATION
            } else {
                [OCR_RESULT, "neuro.official/ocr.codes.v1"].contains(&a.type_id())
            })
            || !catalog
                .visual_declarations("dataTypes")
                .any(|d| d.plugin == plugin && d.id == a.type_id())
        {
            continue;
        }
        if renderer
            .as_ref()
            .is_some_and(|d| d.source && !source_current(store, item, revision))
        {
            continue;
        }
        ids.push(a.id().to_owned());
    }
    if ids.is_empty() {
        return Ok(false);
    }
    let show = !project(Some(catalog), store, revision)
        .iter()
        .any(|v| ids.contains(&v.attachment));
    let mut updates: Vec<_> = ids.iter().map(|id| (id.as_str(), show)).collect();
    if show && !translation {
        updates.extend(replacements.iter().map(|id| (id.as_str(), false)));
    }
    store.visibility(&updates)?;
    Ok(true)
}
#[cfg(any(test, feature = "test-support"))]
#[path = "extension_visual_availability_tests.rs"]
pub mod availability_tests;
#[cfg(test)]
#[path = "extension_visual_developer_tests.rs"]
mod developer_tests;
#[cfg(any(test, feature = "test-support"))]
#[path = "extension_visual_tests.rs"]
pub mod tests;
