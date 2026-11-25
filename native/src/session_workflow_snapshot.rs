//! Prepare hook-live snapshots from saved sessions; no saving, baking, network sends, or inferred authorization.
use crate::session_window::Window;
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};

pub const WORKFLOW_ID: &str = "hook-live";
#[path = "session_workflow_batch.rs"]
pub mod batch;
#[path = "session_workflow_components.rs"]
pub mod components;
#[path = "session_workflow_content.rs"]
pub mod content;
#[path = "session_workflow_metadata.rs"]
pub mod metadata;
#[path = "session_workflow_sources.rs"]
pub mod sources;
#[path = "session_workflow_wire.rs"]
mod wire;
#[derive(Debug, PartialEq)]
pub enum Error {
    Session,
    Revision,
    Node,
    Content,
    Cancelled,
    Budget,
    RequestIdentity,
}

/// Callers filter parameters by current capability and bake edited pins according to mainline rules.
/// Image fields use an allowlist; unknown fields must not enter sync payloads.
pub struct Content {
    pub parameters: Map<String, Value>,
    pub image: Map<String, Value>,
}
pub struct Snapshot {
    source_digest: [u8; 32],
    revision: u64,
    value: Value,
}
impl Snapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn value(&self) -> &Value {
        &self.value
    }
    /// Generate sync_workflow payload only; upper layers still reconfirm authorization and committed-session identity.
    pub fn encode_payload(&self, limit: usize, cancel: &AtomicBool) -> Result<Vec<u8>, Error> {
        encode_payload(WORKFLOW_ID, &self.value, limit, cancel)
    }
    /// Loom control request, not a frontend action wrapper; the send owner allocates and maintains unique requestId values.
    pub fn encode_request(
        &self,
        request_id: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Error> {
        wire::request(WORKFLOW_ID, &self.value, request_id, limit, cancel)
    }
}
fn encode_payload<T: serde::Serialize>(
    workflow: &str,
    snapshot: &T,
    limit: usize,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, Error> {
    #[derive(serde::Serialize)]
    struct Payload<'a, T> {
        workflow_id: &'a str,
        snapshot: &'a T,
    }
    wire::encode(
        &Payload {
            workflow_id: workflow,
            snapshot,
        },
        limit,
        cancel,
    )
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

/// Accept only the caller-specified saved version; successful candidate generation does not prove persistence.
pub fn prepare(
    saved: &[u8],
    expected_revision: u64,
    cancel: &AtomicBool,
    mut content: impl FnMut(&Window<'_>, &Map<String, Value>) -> Result<Content, Error>,
) -> Result<Snapshot, Error> {
    check(cancel)?;
    let document = super::decode(saved).map_err(|_| Error::Session)?;
    prepare_from(&document, expected_revision, cancel, |_, window, fields| {
        content(window, fields)
    })
}

pub(super) fn prepare_from(
    document: &super::Document<'_>,
    expected_revision: u64,
    cancel: &AtomicBool,
    mut content: impl FnMut(usize, &Window<'_>, &Map<String, Value>) -> Result<Content, Error>,
) -> Result<Snapshot, Error> {
    check(cancel)?;
    let saved = document.original_json();
    if expected_revision == 0
        || expected_revision > 9_007_199_254_740_991
        || document.revision != expected_revision
    {
        return Err(Error::Revision);
    }
    let mut root: Map<String, Value> = serde_json::from_slice(saved).map_err(|_| Error::Session)?;
    let Some(Value::Array(records)) = root.remove("stickers") else {
        return Err(Error::Session);
    };
    if records.len() != document.windows.len() {
        return Err(Error::Session);
    }
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(records.len())
        .map_err(|_| Error::Budget)?;
    for (index, (window, record)) in document.windows.iter().zip(records).enumerate() {
        check(cancel)?;
        let Value::Object(mut fields) = record else {
            return Err(Error::Node);
        };
        if fields.get("id").and_then(Value::as_str) != Some(&window.id) {
            return Err(Error::Node);
        }
        if window.is_art()
            && window
                .art_id
                .as_deref()
                .is_none_or(|id| id.trim().is_empty())
        {
            return Err(Error::Node);
        }
        let prepared = content(index, window, &fields)?;
        check(cancel)?;
        if prepared.image.keys().any(|key| {
            !matches!(
                key.as_str(),
                "src" | "filePath" | "previewSrc" | "rasterizedAnnotationLayerSrc"
            )
        }) || prepared
            .image
            .values()
            .any(|value| !value.is_string() && !value.is_null())
        {
            return Err(Error::Content);
        }
        let mut data = prepared.image;
        data.insert(
            "label".into(),
            window.art_id.as_deref().unwrap_or("Node").into(),
        );
        if window.is_art() {
            data.insert("artId".into(), window.art_id.as_deref().unwrap().into());
        }
        data.insert("params".into(), prepared.parameters.into());
        data.insert(
            "outputs".into(),
            fields.remove("outputs").unwrap_or(Value::Null),
        );
        data.insert("w".into(), window.w.into());
        data.insert("h".into(), window.h.into());
        for key in [
            "minified",
            "savedRect",
            "cropOffset",
            "opacityNormal",
            "opacityMini",
            "executionConfig",
        ] {
            if let Some(value) = fields.remove(key) {
                data.insert(key.into(), value);
            }
        }
        // Large image fields already own String; move directly into Value instead of reserializing/copying through json!.
        let kind = if window.is_art() {
            "artNode"
        } else {
            "sticker"
        };
        nodes.push(Value::Object(Map::from_iter([
            ("id".into(), window.id.as_str().into()),
            ("type".into(), kind.into()),
            ("position".into(), json!({"x":window.x,"y":window.y})),
            ("data".into(), Value::Object(data)),
            (
                "measured".into(),
                json!({"width":window.w,"height":window.h}),
            ),
        ])));
    }
    let edges: Vec<_> = document.workspace.links().iter().map(|link| json!({
        "id":link.id.as_str(),"source":link.from_unit.as_str(),"target":link.to_unit.as_str(),
        "sourceHandle":if link.from_port.is_empty(){"output"}else{link.from_port.as_str()},
        "targetHandle":if link.to_port.is_empty(){"input"}else{link.to_port.as_str()}
    })).collect();
    check(cancel)?;
    Ok(Snapshot {
        source_digest: {
            use sha2::Digest;
            sha2::Sha256::digest(saved).into()
        },
        revision: expected_revision,
        value: Value::Object(Map::from_iter([
            ("nodes".into(), Value::Array(nodes)),
            ("edges".into(), Value::Array(edges)),
            ("viewport".into(), json!({"x":0,"y":0,"zoom":1})),
            ("documentSchemaVersion".into(), 1.into()),
            ("documentRevision".into(), expected_revision.into()),
        ])),
    })
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_snapshot_tests.rs"]
mod tests;
