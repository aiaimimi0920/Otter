//! 收藏与回收共用当前编码窗口的冻结边界；只借用原文，不复制图片或运行时所有权。
use super::candidate;
use crate::{session_document, workspace_id::UnitId};
use serde::Serialize;
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Entry<'a> {
    entry_id: &'a str,
    source_sticker_id: &'a str,
    created_at: &'a str,
    snapshot: BTreeMap<String, Cow<'a, RawValue>>,
}
pub(super) fn capture<'a>(
    snapshot: &'a [u8],
    unit: &'a UnitId,
    entry_id: &'a str,
    created_at: &'a str,
) -> Result<Entry<'a>, String> {
    if UnitId::parse(entry_id).is_none()
        || created_at.is_empty()
        || created_at.len() > 128
        || created_at.chars().any(char::is_control)
    {
        return Err("Invalid frozen identity or timestamp".into());
    }
    let document = session_document::decode(snapshot)
        .map_err(|error| format!("Invalid frozen snapshot: {error:?}"))?;
    let index = document
        .windows
        .iter()
        .position(|window| window.id == unit.as_str())
        .ok_or("Frozen source sticker is missing")?;
    if document.windows[index]
        .node_type
        .as_deref()
        .is_some_and(|kind| kind != "sticker")
    {
        return Err("Only stickers can be frozen".into());
    }
    #[derive(serde::Deserialize)]
    struct Windows<'a> {
        #[serde(borrow)]
        stickers: Vec<&'a RawValue>,
    }
    // Document 已验证数组预算；这里再次解析只借用值，不复制图片字符串。
    let windows: Windows = serde_json::from_slice(snapshot).map_err(|error| error.to_string())?;
    let mut snapshot = candidate::fields(windows.stickers[index].get())?;
    for key in [
        "groupId",
        "artId",
        "params",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
    ] {
        snapshot.remove(key);
    }
    Ok(Entry {
        entry_id,
        source_sticker_id: unit.as_str(),
        created_at,
        snapshot,
    })
}
