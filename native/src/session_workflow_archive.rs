//! Archive-index candidates for published image references; no resource/session writes or document-revision advancement.
use super::{check, Error, Plan};
use serde::Deserialize;
use serde_json::{json, value::RawValue, Map, Value};
use std::{borrow::Cow, collections::BTreeMap, sync::atomic::AtomicBool};

pub struct Asset<'a> {
    pub sticker_id: &'a str,
    pub src: &'a str,
    pub preview_src: Option<&'a str>,
}
/// Bind exact saved source content. Borrow JSON when parsing image references and original indexes, without allocating large unescaped strings.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fields<'a> {
    #[serde(borrow, deserialize_with = "stickers")]
    stickers: Vec<Sticker<'a>>,
    #[serde(borrow)]
    workflow_asset_archive_index: Option<&'a RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Sticker<'a> {
    #[serde(borrow)]
    pub id: Cow<'a, str>,
    #[serde(default, borrow)]
    pub src: Cow<'a, str>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub preview_src: Option<Cow<'a, str>>,
}
fn stickers<'de: 'a, 'a, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<Sticker<'a>>, D::Error> {
    crate::session_document::bounded::<D, Sticker<'a>, { crate::workspace::MAX_UNITS }>(decoder)
}

pub(crate) struct Source<'a> {
    plan: &'a Plan,
    fields: Fields<'a>,
}
impl<'a> Source<'a> {
    pub fn read(plan: &'a Plan, saved: &'a [u8], cancel: &AtomicBool) -> Result<Self, Error> {
        check(cancel)?;
        if !plan.matches(saved) {
            return Err(Error::Revision);
        }
        let fields = serde_json::from_slice(saved).map_err(|_| Error::Session)?;
        check(cancel)?;
        Ok(Self { plan, fields })
    }
    pub fn take_stickers(&mut self) -> Vec<Sticker<'a>> {
        std::mem::take(&mut self.fields.stickers)
    }
    /// Use final references after successful resource publication; indexes never change their bound save baseline.
    pub fn index(
        self,
        assets: &[Asset<'_>],
        timestamp_ms: u64,
        cancel: &AtomicBool,
    ) -> Result<Value, Error> {
        check(cancel)?;
        if assets.len() > crate::workspace::MAX_UNITS {
            return Err(Error::Budget);
        }
        let index = self
            .fields
            .workflow_asset_archive_index
            .map(|raw| serde_json::from_str(raw.get()).map_err(|_| Error::Session))
            .transpose()?
            .unwrap_or_default();
        build_index(self.plan, index, assets, timestamp_ms, cancel)
    }
}

fn build_index(
    plan: &Plan,
    mut index: Map<String, Value>,
    assets: &[Asset<'_>],
    timestamp_ms: u64,
    cancel: &AtomicBool,
) -> Result<Value, Error> {
    let version = index
        .get("version")
        .map(Value::as_u64)
        .unwrap_or(Some(0))
        .ok_or(Error::Session)?;
    if version > u32::MAX as u64 {
        return Err(Error::Session);
    }
    index.insert("version".into(), json!(version.max(1)));
    let mut workflows = match index.remove("workflows") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(workflows)) => workflows,
        _ => return Err(Error::Session),
    };
    let mut by_id = BTreeMap::new();
    for asset in assets {
        check(cancel)?;
        if crate::workspace_id::UnitId::parse(asset.sticker_id).is_none() {
            return Err(Error::Node);
        }
        if by_id.insert(asset.sticker_id, asset).is_some() {
            return Err(Error::Node);
        }
        for reference in [Some(asset.src), asset.preview_src].into_iter().flatten() {
            // Inline content must first be published by the resource owner; do not copy large images into every workflow index.
            if reference.starts_with("data:image")
                || reference.len() > 32 * 1024
                || reference.contains('\0')
            {
                return Err(Error::Content);
            }
        }
    }
    let timestamp = timestamp_ms.to_string();
    for (id, hint) in &plan.archive_hints().workflows {
        check(cancel)?;
        let mut nodes = Map::new();
        for (node, hint) in &hint.nodes {
            check(cancel)?;
            let Some(asset) = by_id.get(hint.sticker_id.as_str()) else {
                continue;
            };
            nodes.insert(
                node.clone(),
                json!({
                    "stickerId":asset.sticker_id, "updatedAt":timestamp,
                    "src": if asset.src.is_empty() {None} else {Some(asset.src)},
                    "previewSrc":asset.preview_src,
                }),
            );
        }
        // Match mainline: replace nodes wholesale in matching workflows and preserve unmatched old workflows unchanged.
        workflows.insert(id.clone(), json!({"updatedAt":timestamp,"nodes":nodes}));
    }
    index.insert("workflows".into(), Value::Object(workflows));
    let value = Value::Object(index);
    // A unified encoding limit covers old indexes and new nodes; never carry arbitrarily large legacy fields into the next save.
    struct Count<'a> {
        remaining: usize,
        cancel: &'a AtomicBool,
    }
    impl std::io::Write for Count<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if check(self.cancel).is_err() || bytes.len() > self.remaining {
                return Err(std::io::Error::other(
                    "archive index cancelled or over budget",
                ));
            }
            self.remaining -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let result = serde_json::to_writer(
        Count {
            remaining: crate::session_document::MAX_SESSION_BYTES,
            cancel,
        },
        &value,
    );
    check(cancel)?;
    result.map_err(|_| Error::Budget)?;
    Ok(value)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_archive_tests.rs"]
mod tests;
