//! 冻结条目追加发布前的会话原文候选；仅构造内存，成功提交由窗口 owner 决定。
use super::{candidate, Kind};
use crate::{session_document, workspace_id::UnitId};
use std::sync::atomic::AtomicBool;

/// active 是当前工作区身份集合；移除已关闭旧窗口的原文，避免它们占用窗口预算。
/// 只裁掉随旧窗口失效的原文链接；当前拓扑/几何在后续正式保存时统一采集。
pub fn prepare(
    original: &[u8],
    instance: &[u8],
    active: &[UnitId],
    kind: Kind,
    entry_id: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    let source =
        session_document::decode(original).map_err(|error| format!("Invalid source: {error:?}"))?;
    let copy = session_document::decode(instance)
        .map_err(|error| format!("Invalid instance: {error:?}"))?;
    let library = source.library(kind)?;
    let entry = library
        .find(entry_id)
        .ok_or("Frozen entry no longer exists")?;
    if copy.windows.len() != 1
        || !copy.workspace.links().is_empty()
        || !copy.workspace.groups().is_empty()
    {
        return Err("Frozen publication requires one ungrouped sticker".into());
    }
    let window = &copy.windows[0];
    if window.node_type.as_deref() != Some("sticker")
        || window.group_id.is_some()
        || window.params.is_none_or(|params| params.get() != "{}")
        || window.id == entry.source_sticker_id
        || active.iter().any(|unit| unit.as_str() == window.id)
        || source.windows.iter().any(|old| old.id == window.id)
        || active.len() >= crate::workspace::MAX_UNITS
    {
        return Err("Frozen publication identity or capacity conflict".into());
    }
    let copy_root =
        candidate::fields(std::str::from_utf8(instance).map_err(|error| error.to_string())?)?;
    let new: Vec<&serde_json::value::RawValue> =
        serde_json::from_str(copy_root["stickers"].get()).map_err(|error| error.to_string())?;
    if !matches_snapshot(entry.snapshot_json(), new[0].get())? {
        return Err("Frozen instance does not match selected snapshot".into());
    }
    let recycle = match kind {
        Kind::RecycleBin => Some(
            library
                .entries()
                .iter()
                .filter(|record| record.entry_id != entry_id)
                .map(|record| record.original)
                .collect(),
        ),
        Kind::ReferenceLibrary => None,
    };
    crate::session_append::prepare_with_recycle(original, instance, active, recycle, cancel)
}

// 只允许实例化拥有的身份/位置/空参数差异，防止拿任意窗口消费另一条冻结记录。
fn matches_snapshot(snapshot: &str, instance: &str) -> Result<bool, String> {
    let mut expected = candidate::fields(snapshot)?;
    let mut actual = candidate::fields(instance)?;
    for key in [
        "groupId",
        "artId",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
    ] {
        expected.remove(key);
    }
    for key in ["id", "type", "x", "y", "params"] {
        expected.remove(key);
        actual.remove(key);
    }
    Ok(expected.len() == actual.len()
        && expected.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|actual| actual.get() == value.get())
        }))
}

#[cfg(test)]
#[path = "session_library_retention_tests.rs"]
mod tests;
